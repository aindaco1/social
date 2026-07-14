use crate::domain::{
    AccountForm, TwitterOAuthExchangeForm, TwitterOAuthStartForm, TwitterOAuthStartSummary,
};
use crate::secrets::{SecretError, resolve_service_credential, save_account_secret};
use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};
use reqwest::{
    StatusCode, Url,
    blocking::{Client, multipart},
};
use serde_json::{Value, json};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;
use uuid::Uuid;

const DEFAULT_TWITTER_REDIRECT_URI: &str = "http://localhost/callback";
const DEFAULT_TWITTER_SCOPES: &[&str] =
    &["tweet.read", "tweet.write", "users.read", "offline.access"];
const TWITTER_MEDIA_CHUNK_BYTES: usize = 4 * 1024 * 1024;
const TWITTER_MEDIA_MAX_STATUS_POLLS: usize = 12;
const TWITTER_POST_IMPORT_MAX_PAGES: usize = 5;

#[derive(Debug, Clone)]
pub struct TwitterAccountAuthorization {
    pub account: AccountForm,
}

#[derive(Debug, Clone)]
pub struct TwitterPublishRequest {
    pub access_token: String,
    pub text: String,
    pub media_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TwitterPublishResponse {
    pub id: String,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct TwitterMediaUploadRequest {
    pub access_token: String,
    pub file_path: PathBuf,
    pub mime_type: String,
}

#[derive(Debug, Clone)]
pub struct TwitterMediaUploadResponse {
    pub id: String,
    pub raw: Value,
}

#[derive(Debug)]
pub enum TwitterError {
    Http(reqwest::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Secret(SecretError),
    RateLimited(String),
    Unauthorized(String),
    Validation(String),
}

impl Display for TwitterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "X request failed: {error}"),
            Self::Io(error) => write!(formatter, "X media file error: {error}"),
            Self::Json(error) => write!(formatter, "X response was invalid: {error}"),
            Self::Secret(error) => write!(formatter, "{error}"),
            Self::RateLimited(error) => write!(formatter, "{error}"),
            Self::Unauthorized(error) => write!(formatter, "{error}"),
            Self::Validation(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for TwitterError {}

impl From<reqwest::Error> for TwitterError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

impl From<std::io::Error> for TwitterError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for TwitterError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<SecretError> for TwitterError {
    fn from(error: SecretError) -> Self {
        Self::Secret(error)
    }
}

pub fn start_twitter_oauth(
    form: &TwitterOAuthStartForm,
) -> Result<TwitterOAuthStartSummary, TwitterError> {
    let client_id = resolve_service_credential("twitter", "client_id")?;

    twitter_authorization_summary(&client_id, form)
}

pub fn connect_twitter_account(
    form: &TwitterOAuthExchangeForm,
) -> Result<TwitterAccountAuthorization, TwitterError> {
    let client_id = resolve_service_credential("twitter", "client_id")?;
    let client_secret = resolve_service_credential("twitter", "client_secret")?;
    let code = form.code.trim();
    let code_verifier = form.code_verifier.trim();
    let redirect_uri = normalized_twitter_redirect_uri(form.redirect_uri.as_deref());

    if code.is_empty() {
        return Err(TwitterError::Validation("code is required".to_string()));
    }

    if code_verifier.is_empty() {
        return Err(TwitterError::Validation(
            "code_verifier is required".to_string(),
        ));
    }

    let client = twitter_client(Duration::from_secs(30))?;
    let token_params = [
        ("code", code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri.as_str()),
        ("code_verifier", code_verifier),
    ];
    let token_response = client
        .post("https://api.x.com/2/oauth2/token")
        .basic_auth(&client_id, Some(&client_secret))
        .form(&token_params)
        .send()?;
    let token_status = token_response.status();
    let token_text = token_response.text()?;
    let token_value: Value = serde_json::from_str(&token_text)?;

    if !token_status.is_success() {
        return Err(TwitterError::Validation(twitter_error_message(
            &token_value,
            "X OAuth token exchange failed",
        )));
    }

    let access_token = required_string(
        &token_value,
        "access_token",
        "X response missing access_token",
    )?;
    let me_value = fetch_twitter_me_with_client(&client, &access_token)?;
    let account = twitter_account_form(&me_value, &token_value, access_token)?;

    Ok(TwitterAccountAuthorization { account })
}

pub fn verify_twitter_account(
    access_token: &str,
    access_token_secret_ref: String,
    existing_data_json: Option<&str>,
) -> Result<AccountForm, TwitterError> {
    let access_token = access_token.trim();

    if access_token.is_empty() {
        return Err(TwitterError::Validation(
            "X access token is required".to_string(),
        ));
    }

    let client = twitter_client(Duration::from_secs(30))?;
    let me_value = fetch_twitter_me_with_client(&client, access_token)?;

    twitter_verified_account_form(&me_value, access_token_secret_ref, existing_data_json)
}

pub fn fetch_twitter_account_metrics(access_token: &str) -> Result<Value, TwitterError> {
    let access_token = access_token.trim();

    if access_token.is_empty() {
        return Err(TwitterError::Validation(
            "X access token is required".to_string(),
        ));
    }

    let client = twitter_client(Duration::from_secs(30))?;
    let me_value = fetch_twitter_me_with_client(&client, access_token)?;

    Ok(me_value
        .get("data")
        .and_then(|data| data.get("public_metrics"))
        .cloned()
        .unwrap_or(Value::Null))
}

pub fn fetch_twitter_user_posts(
    access_token: &str,
    user_id: &str,
) -> Result<Vec<Value>, TwitterError> {
    let access_token = access_token.trim();
    let user_id = user_id.trim();

    if access_token.is_empty() {
        return Err(TwitterError::Validation(
            "X access token is required".to_string(),
        ));
    }

    if user_id.is_empty() {
        return Err(TwitterError::Validation(
            "X user id is required".to_string(),
        ));
    }

    let client = twitter_client(Duration::from_secs(30))?;
    let endpoint = format!("https://api.x.com/2/users/{user_id}/tweets");
    let start_time =
        (Utc::now() - ChronoDuration::days(90)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut posts = Vec::new();
    let mut pagination_token: Option<String> = None;

    for _ in 0..TWITTER_POST_IMPORT_MAX_PAGES {
        let mut query = vec![
            (
                "tweet.fields".to_string(),
                "public_metrics,created_at".to_string(),
            ),
            ("exclude".to_string(), "retweets,replies".to_string()),
            ("max_results".to_string(), "100".to_string()),
            ("start_time".to_string(), start_time.clone()),
        ];

        if let Some(token) = pagination_token.as_ref() {
            query.push(("pagination_token".to_string(), token.clone()));
        }

        let response = client
            .get(&endpoint)
            .bearer_auth(access_token)
            .query(&query)
            .send()?;
        let status = response.status();
        let text = response.text()?;
        let value: Value = serde_json::from_str(&text)?;

        if !status.is_success() {
            return Err(twitter_http_error(
                status,
                &value,
                "X user posts import failed",
            ));
        }

        if let Some(data) = value.get("data").and_then(Value::as_array) {
            posts.extend(data.iter().cloned());
        }

        pagination_token = value
            .get("meta")
            .and_then(|meta| meta.get("next_token"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        if pagination_token.is_none() {
            break;
        }
    }

    Ok(posts)
}

pub fn publish_twitter_post(
    request: &TwitterPublishRequest,
) -> Result<TwitterPublishResponse, TwitterError> {
    let access_token = request.access_token.trim();

    if access_token.is_empty() {
        return Err(TwitterError::Validation(
            "X access token is required".to_string(),
        ));
    }

    let payload = twitter_post_payload(&request.text, &request.media_ids)?;

    let client = twitter_client(Duration::from_secs(30))?;
    let response = client
        .post("https://api.x.com/2/tweets")
        .bearer_auth(access_token)
        .json(&payload)
        .send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(twitter_http_error(status, &value, "X post publish failed"));
    }

    twitter_publish_response(value)
}

pub fn upload_twitter_media(
    request: &TwitterMediaUploadRequest,
) -> Result<TwitterMediaUploadResponse, TwitterError> {
    let access_token = request.access_token.trim();
    let mime_type = normalized_media_mime_type(&request.mime_type);
    let category = twitter_media_category(&mime_type)?;

    if access_token.is_empty() {
        return Err(TwitterError::Validation(
            "X access token is required".to_string(),
        ));
    }

    if !request.file_path.exists() {
        return Err(TwitterError::Validation(format!(
            "X media file not found: {}",
            request.file_path.display()
        )));
    }

    let client = twitter_client(Duration::from_secs(60))?;

    if category == "tweet_image" {
        return upload_twitter_image(&client, access_token, &request.file_path, &mime_type);
    }

    upload_twitter_chunked_media(
        &client,
        access_token,
        &request.file_path,
        &mime_type,
        category,
    )
}

fn twitter_authorization_summary(
    client_id: &str,
    form: &TwitterOAuthStartForm,
) -> Result<TwitterOAuthStartSummary, TwitterError> {
    let client_id = client_id.trim();

    if client_id.is_empty() {
        return Err(TwitterError::Validation(
            "Twitter client_id is required".to_string(),
        ));
    }

    let redirect_uri = normalized_twitter_redirect_uri(form.redirect_uri.as_deref());
    let scopes = normalized_twitter_scopes(&form.scopes);
    let state = Uuid::new_v4().to_string();
    let code_verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let mut url = Url::parse("https://x.com/i/oauth2/authorize")
        .map_err(|error| TwitterError::Validation(format!("invalid X authorize URL: {error}")))?;

    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", &scopes.join(" "))
        .append_pair("state", &state)
        .append_pair("code_challenge", &code_verifier)
        .append_pair("code_challenge_method", "plain");

    Ok(TwitterOAuthStartSummary {
        auth_url: url.to_string(),
        code_verifier,
        state,
        redirect_uri,
        scopes,
    })
}

fn twitter_client(timeout: Duration) -> Result<Client, TwitterError> {
    Client::builder()
        .user_agent("DustWaveSocial/0.1")
        .timeout(timeout)
        .build()
        .map_err(TwitterError::Http)
}

fn fetch_twitter_me_with_client(
    client: &Client,
    access_token: &str,
) -> Result<Value, TwitterError> {
    let response = client
        .get("https://api.x.com/2/users/me")
        .bearer_auth(access_token)
        .query(&[("user.fields", "profile_image_url,public_metrics")])
        .send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(twitter_http_error(
            status,
            &value,
            "X authenticated user lookup failed",
        ));
    }

    Ok(value)
}

fn twitter_account_form(
    me_value: &Value,
    token_value: &Value,
    access_token: String,
) -> Result<AccountForm, TwitterError> {
    let data = me_value
        .get("data")
        .ok_or_else(|| TwitterError::Validation("X response missing user data".to_string()))?;
    let provider_id = required_string(data, "id", "X response missing user id")?;
    let name = required_string(data, "name", "X response missing user name")?;
    let username = required_string(data, "username", "X response missing username")?;
    let access_token_secret_ref =
        save_account_secret("twitter", &provider_id, "access_token", &access_token)?;
    let refresh_token_ref = token_value
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|refresh_token| {
            save_account_secret("twitter", &provider_id, "refresh_token", refresh_token)
        })
        .transpose()?;

    Ok(AccountForm {
        name,
        username: Some(username),
        provider: "twitter".to_string(),
        provider_id,
        authorized: true,
        avatar_path: optional_string(data, "profile_image_url"),
        access_token_secret_ref,
        data: Some(json!({
            "auth": "oauth2_pkce",
            "scope": optional_string(token_value, "scope"),
            "token_type": optional_string(token_value, "token_type"),
            "expires_in": token_value.get("expires_in").cloned(),
            "refresh_token_ref": refresh_token_ref,
            "public_metrics": data.get("public_metrics").cloned(),
        })),
    })
}

fn twitter_verified_account_form(
    me_value: &Value,
    access_token_secret_ref: String,
    existing_data_json: Option<&str>,
) -> Result<AccountForm, TwitterError> {
    let data = me_value
        .get("data")
        .ok_or_else(|| TwitterError::Validation("X response missing user data".to_string()))?;
    let provider_id = required_string(data, "id", "X response missing user id")?;
    let name = required_string(data, "name", "X response missing user name")?;
    let username = required_string(data, "username", "X response missing username")?;
    let mut metadata = existing_data_json
        .and_then(|value| serde_json::from_str::<Value>(value).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();

    metadata.insert("auth".to_string(), Value::String("oauth2_pkce".to_string()));
    metadata.insert(
        "public_metrics".to_string(),
        data.get("public_metrics").cloned().unwrap_or(Value::Null),
    );

    Ok(AccountForm {
        name,
        username: Some(username),
        provider: "twitter".to_string(),
        provider_id,
        authorized: true,
        avatar_path: optional_string(data, "profile_image_url"),
        access_token_secret_ref,
        data: Some(Value::Object(metadata)),
    })
}

fn upload_twitter_image(
    client: &Client,
    access_token: &str,
    file_path: &Path,
    mime_type: &str,
) -> Result<TwitterMediaUploadResponse, TwitterError> {
    let bytes = fs::read(file_path)?;
    let part = multipart::Part::bytes(bytes)
        .file_name(media_file_name(file_path))
        .mime_str(mime_type)
        .map_err(|error| TwitterError::Validation(format!("invalid media MIME type: {error}")))?;
    let form = multipart::Form::new()
        .part("media", part)
        .text("media_category", "tweet_image")
        .text("media_type", mime_type.to_string());
    let value = send_twitter_media_form(
        client,
        access_token,
        "https://api.x.com/2/media/upload",
        form,
        "X media upload failed",
    )?;

    twitter_media_upload_response(value)
}

fn upload_twitter_chunked_media(
    client: &Client,
    access_token: &str,
    file_path: &Path,
    mime_type: &str,
    category: &str,
) -> Result<TwitterMediaUploadResponse, TwitterError> {
    let size = fs::metadata(file_path)?.len();
    let init_form = multipart::Form::new()
        .text("command", "INIT")
        .text("media_type", mime_type.to_string())
        .text("total_bytes", size.to_string())
        .text("media_category", category.to_string());
    let init_value = send_twitter_media_form(
        client,
        access_token,
        "https://api.x.com/2/media/upload",
        init_form,
        "X media upload initialization failed",
    )?;
    let init_response = twitter_media_upload_response(init_value)?;
    let mut file = fs::File::open(file_path)?;
    let mut segment_index = 0;

    loop {
        let mut buffer = vec![0; TWITTER_MEDIA_CHUNK_BYTES];
        let bytes_read = file.read(&mut buffer)?;

        if bytes_read == 0 {
            break;
        }

        buffer.truncate(bytes_read);

        let part = multipart::Part::bytes(buffer)
            .file_name(media_file_name(file_path))
            .mime_str(mime_type)
            .map_err(|error| {
                TwitterError::Validation(format!("invalid media MIME type: {error}"))
            })?;
        let append_form = multipart::Form::new()
            .text("command", "APPEND")
            .text("media_id", init_response.id.clone())
            .text("segment_index", segment_index.to_string())
            .part("media", part);

        send_twitter_optional_media_form(
            client,
            access_token,
            "https://api.x.com/2/media/upload",
            append_form,
            "X media chunk upload failed",
        )?;

        segment_index += 1;
    }

    let finalize_form = multipart::Form::new()
        .text("command", "FINALIZE")
        .text("media_id", init_response.id.clone());
    let finalize_value = send_twitter_media_form(
        client,
        access_token,
        "https://api.x.com/2/media/upload",
        finalize_form,
        "X media upload finalization failed",
    )?;
    let ready_value = wait_for_twitter_media(client, access_token, finalize_value)?;

    twitter_media_upload_response(ready_value)
}

fn wait_for_twitter_media(
    client: &Client,
    access_token: &str,
    initial: Value,
) -> Result<Value, TwitterError> {
    let mut current = initial;

    if twitter_media_upload_ready(&current)? {
        return Ok(current);
    }

    let media_id = twitter_media_upload_id(&current)?;

    for _ in 0..TWITTER_MEDIA_MAX_STATUS_POLLS {
        let seconds = twitter_media_check_after_seconds(&current).unwrap_or(1);

        sleep(Duration::from_secs(seconds));

        let response = client
            .get("https://api.x.com/2/media/upload")
            .bearer_auth(access_token)
            .query(&[("command", "STATUS"), ("media_id", media_id.as_str())])
            .send()?;
        let status = response.status();
        let text = response.text()?;
        current = serde_json::from_str(&text)?;

        if !status.is_success() {
            return Err(twitter_http_error(
                status,
                &current,
                "X media processing check failed",
            ));
        }

        if twitter_media_upload_ready(&current)? {
            return Ok(current);
        }
    }

    Err(TwitterError::Validation(
        "X media upload is still processing".to_string(),
    ))
}

fn send_twitter_media_form(
    client: &Client,
    access_token: &str,
    endpoint: &str,
    form: multipart::Form,
    fallback: &str,
) -> Result<Value, TwitterError> {
    let value = send_twitter_optional_media_form(client, access_token, endpoint, form, fallback)?
        .ok_or_else(|| TwitterError::Validation(format!("{fallback}: empty response")))?;

    Ok(value)
}

fn send_twitter_optional_media_form(
    client: &Client,
    access_token: &str,
    endpoint: &str,
    form: multipart::Form,
    fallback: &str,
) -> Result<Option<Value>, TwitterError> {
    let response = client
        .post(endpoint)
        .bearer_auth(access_token)
        .multipart(form)
        .send()?;
    let status = response.status();
    let text = response.text()?;
    let value = if text.trim().is_empty() {
        None
    } else {
        Some(serde_json::from_str::<Value>(&text)?)
    };

    if !status.is_success() {
        return Err(value
            .as_ref()
            .map(|value| twitter_http_error(status, value, fallback))
            .unwrap_or_else(|| TwitterError::Validation(fallback.to_string())));
    }

    if let Some(value) = value.as_ref() {
        if value
            .get("errors")
            .and_then(Value::as_array)
            .is_some_and(|errors| !errors.is_empty())
        {
            return Err(TwitterError::Validation(twitter_error_message(
                value, fallback,
            )));
        }
    }

    Ok(value)
}

fn twitter_post_payload(text: &str, media_ids: &[String]) -> Result<Value, TwitterError> {
    let text = text.trim();
    let media_ids = media_ids
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut payload = serde_json::Map::new();

    if !text.is_empty() {
        payload.insert("text".to_string(), Value::String(text.to_string()));
    }

    if !media_ids.is_empty() {
        payload.insert("media".to_string(), json!({ "media_ids": media_ids }));
    }

    if payload.is_empty() {
        return Err(TwitterError::Validation(
            "X post text or media is required".to_string(),
        ));
    }

    Ok(Value::Object(payload))
}

fn twitter_publish_response(value: Value) -> Result<TwitterPublishResponse, TwitterError> {
    let data = value
        .get("data")
        .ok_or_else(|| TwitterError::Validation("X response missing post data".to_string()))?;
    let id = required_string(data, "id", "X response missing post id")?;

    Ok(TwitterPublishResponse { id, raw: value })
}

fn twitter_media_upload_response(value: Value) -> Result<TwitterMediaUploadResponse, TwitterError> {
    let id = twitter_media_upload_id(&value)?;

    Ok(TwitterMediaUploadResponse { id, raw: value })
}

fn twitter_media_upload_id(value: &Value) -> Result<String, TwitterError> {
    let data = value
        .get("data")
        .ok_or_else(|| TwitterError::Validation("X response missing media data".to_string()))?;

    optional_string(data, "id")
        .or_else(|| optional_string(data, "media_id_string"))
        .ok_or_else(|| TwitterError::Validation("X response missing media id".to_string()))
}

fn twitter_media_upload_ready(value: &Value) -> Result<bool, TwitterError> {
    let Some(processing_info) = value
        .get("data")
        .and_then(|data| data.get("processing_info"))
    else {
        return Ok(true);
    };
    let state = optional_string(processing_info, "state");

    match state.as_deref() {
        Some("succeeded") => Ok(true),
        Some("failed") => Err(TwitterError::Validation(
            twitter_processing_error(processing_info)
                .unwrap_or_else(|| "X media processing failed".to_string()),
        )),
        Some("pending") | Some("in_progress") => Ok(false),
        Some(_) => Ok(false),
        None => Ok(processing_info
            .get("progress_percent")
            .and_then(Value::as_i64)
            .is_some_and(|percent| percent >= 100)),
    }
}

fn twitter_processing_error(value: &Value) -> Option<String> {
    value
        .get("error")
        .and_then(|error| {
            error
                .get("message")
                .and_then(Value::as_str)
                .or_else(|| error.get("name").and_then(Value::as_str))
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn twitter_media_check_after_seconds(value: &Value) -> Option<u64> {
    value
        .get("data")
        .and_then(|data| data.get("processing_info"))
        .and_then(|processing| processing.get("check_after_secs"))
        .and_then(Value::as_u64)
        .map(|seconds| seconds.clamp(1, 5))
}

fn normalized_media_mime_type(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "image/jpg" => "image/jpeg".to_string(),
        "video/x-m4v" => "video/mp4".to_string(),
        value => value.to_string(),
    }
}

fn twitter_media_category(mime_type: &str) -> Result<&'static str, TwitterError> {
    if mime_type == "image/gif" {
        return Ok("tweet_gif");
    }

    if mime_type.starts_with("image/") {
        return Ok("tweet_image");
    }

    if mime_type.starts_with("video/") {
        return Ok("tweet_video");
    }

    Err(TwitterError::Validation(format!(
        "X does not support media MIME type {mime_type}"
    )))
}

fn media_file_name(file_path: &Path) -> String {
    file_path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("media")
        .to_string()
}

fn normalized_twitter_redirect_uri(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_TWITTER_REDIRECT_URI)
        .to_string()
}

fn normalized_twitter_scopes(scopes: &[String]) -> Vec<String> {
    let scopes = scopes
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if scopes.is_empty() {
        return DEFAULT_TWITTER_SCOPES
            .iter()
            .map(|value| value.to_string())
            .collect();
    }

    scopes
}

fn twitter_error_message(value: &Value, fallback: &str) -> String {
    value
        .get("error_description")
        .and_then(Value::as_str)
        .or_else(|| value.get("error").and_then(Value::as_str))
        .or_else(|| value.get("detail").and_then(Value::as_str))
        .or_else(|| {
            value
                .get("errors")
                .and_then(Value::as_array)
                .and_then(|errors| errors.first())
                .and_then(|error| {
                    error
                        .get("detail")
                        .and_then(Value::as_str)
                        .or_else(|| error.get("message").and_then(Value::as_str))
                        .or_else(|| error.get("title").and_then(Value::as_str))
                })
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn twitter_http_error(status: StatusCode, value: &Value, fallback: &str) -> TwitterError {
    let message = twitter_error_message(value, fallback);

    match status {
        StatusCode::UNAUTHORIZED => TwitterError::Unauthorized(message),
        StatusCode::TOO_MANY_REQUESTS => TwitterError::RateLimited(message),
        _ => TwitterError::Validation(message),
    }
}

fn required_string(value: &Value, key: &str, message: &str) -> Result<String, TwitterError> {
    optional_string(value, key).ok_or_else(|| TwitterError::Validation(message.to_string()))
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_twitter_authorization_url() {
        let summary = twitter_authorization_summary(
            "client-id",
            &TwitterOAuthStartForm {
                redirect_uri: Some(" http://localhost/callback ".to_string()),
                scopes: Vec::new(),
            },
        )
        .expect("summary should build");

        assert!(
            summary
                .auth_url
                .starts_with("https://x.com/i/oauth2/authorize?")
        );
        assert!(summary.auth_url.contains("response_type=code"));
        assert!(summary.auth_url.contains("client_id=client-id"));
        assert!(
            summary
                .auth_url
                .contains("redirect_uri=http%3A%2F%2Flocalhost%2Fcallback")
        );
        assert!(
            summary
                .auth_url
                .contains("scope=tweet.read+tweet.write+users.read+offline.access")
        );
        assert!(summary.auth_url.contains("code_challenge_method=plain"));
        assert_eq!(summary.code_verifier.len(), 64);
    }

    #[test]
    fn rejects_token_responses_without_access_token() {
        let error = required_string(
            &serde_json::json!({ "token_type": "bearer" }),
            "access_token",
            "X response missing access_token",
        )
        .expect_err("missing access token should fail")
        .to_string();

        assert_eq!(error, "X response missing access_token");
    }

    #[test]
    fn maps_twitter_publish_response() {
        let response = twitter_publish_response(serde_json::json!({
            "data": {
                "id": "123",
                "text": "Launch"
            }
        }))
        .expect("publish response should map");

        assert_eq!(response.id, "123");
        assert_eq!(
            response
                .raw
                .get("data")
                .and_then(|data| data.get("text"))
                .and_then(Value::as_str),
            Some("Launch")
        );
    }

    #[test]
    fn builds_twitter_post_payload_with_media() {
        let payload = twitter_post_payload(
            "  Launch clip  ",
            &[
                " media-1 ".to_string(),
                "".to_string(),
                "media-2".to_string(),
            ],
        )
        .expect("payload should build");

        assert_eq!(
            payload.get("text").and_then(Value::as_str),
            Some("Launch clip")
        );
        assert_eq!(
            payload
                .get("media")
                .and_then(|media| media.get("media_ids"))
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(2)
        );
    }

    #[test]
    fn rejects_empty_twitter_post_payload() {
        let error = twitter_post_payload("   ", &[])
            .expect_err("empty payload should fail")
            .to_string();

        assert_eq!(error, "X post text or media is required");
    }

    #[test]
    fn maps_twitter_media_upload_response() {
        let response = twitter_media_upload_response(serde_json::json!({
            "data": {
                "id": "media-123",
                "media_key": "7_123"
            }
        }))
        .expect("media response should map");

        assert_eq!(response.id, "media-123");
        assert_eq!(
            response
                .raw
                .get("data")
                .and_then(|data| data.get("media_key"))
                .and_then(Value::as_str),
            Some("7_123")
        );
    }

    #[test]
    fn rejects_twitter_media_upload_response_without_id() {
        let error = twitter_media_upload_response(serde_json::json!({
            "data": {
                "media_key": "7_123"
            }
        }))
        .expect_err("missing media id should fail")
        .to_string();

        assert_eq!(error, "X response missing media id");
    }

    #[test]
    fn detects_twitter_media_processing_states() {
        assert!(
            twitter_media_upload_ready(&serde_json::json!({
                "data": {
                    "id": "media-123"
                }
            }))
            .expect("media without processing should be ready")
        );
        assert!(
            !twitter_media_upload_ready(&serde_json::json!({
                "data": {
                    "id": "media-123",
                    "processing_info": {
                        "state": "in_progress",
                        "check_after_secs": 1
                    }
                }
            }))
            .expect("in-progress media should not be ready")
        );

        let error = twitter_media_upload_ready(&serde_json::json!({
            "data": {
                "id": "media-123",
                "processing_info": {
                    "state": "failed",
                    "error": {
                        "message": "codec not supported"
                    }
                }
            }
        }))
        .expect_err("failed media processing should fail")
        .to_string();

        assert_eq!(error, "codec not supported");
    }

    #[test]
    fn maps_twitter_media_categories() {
        assert_eq!(normalized_media_mime_type("image/jpg"), "image/jpeg");
        assert_eq!(normalized_media_mime_type("video/x-m4v"), "video/mp4");
        assert_eq!(
            twitter_media_category("image/jpeg").expect("jpeg should map"),
            "tweet_image"
        );
        assert_eq!(
            twitter_media_category("image/gif").expect("gif should map"),
            "tweet_gif"
        );
        assert_eq!(
            twitter_media_category("video/mp4").expect("video should map"),
            "tweet_video"
        );
    }

    #[test]
    fn maps_verified_twitter_account_form() {
        let form = twitter_verified_account_form(
            &serde_json::json!({
                "data": {
                    "id": "42",
                    "name": "Dust Wave",
                    "username": "dustwave",
                    "profile_image_url": "https://example.test/avatar.jpg",
                    "public_metrics": {
                        "followers_count": 100,
                        "tweet_count": 20
                    }
                }
            }),
            "secret://accounts/twitter/42/access_token".to_string(),
            Some(r#"{"auth":"oauth2_pkce","refresh_token_ref":"secret://refresh"}"#),
        )
        .expect("account form should map");

        assert_eq!(form.provider, "twitter");
        assert_eq!(form.provider_id, "42");
        assert_eq!(form.username.as_deref(), Some("dustwave"));
        assert_eq!(
            form.data
                .as_ref()
                .and_then(|data| data.get("refresh_token_ref"))
                .and_then(Value::as_str),
            Some("secret://refresh")
        );
        assert_eq!(
            form.data
                .as_ref()
                .and_then(|data| data.get("public_metrics"))
                .and_then(|metrics| metrics.get("followers_count"))
                .and_then(Value::as_i64),
            Some(100)
        );
    }

    #[test]
    fn maps_twitter_http_errors_by_status() {
        assert!(matches!(
            twitter_http_error(
                StatusCode::UNAUTHORIZED,
                &serde_json::json!({ "detail": "bad token" }),
                "fallback"
            ),
            TwitterError::Unauthorized(_)
        ));
        assert!(matches!(
            twitter_http_error(
                StatusCode::TOO_MANY_REQUESTS,
                &serde_json::json!({ "errors": [{ "detail": "wait" }] }),
                "fallback"
            ),
            TwitterError::RateLimited(_)
        ));
    }
}
