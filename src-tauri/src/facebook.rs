use crate::domain::{
    AccountForm, FacebookOAuthExchangeForm, FacebookOAuthStartForm, FacebookOAuthStartSummary,
    FacebookPageCandidate, FacebookPageConnectForm,
};
use crate::secrets::{
    SecretError, resolve_account_secret, resolve_service_credential, save_account_secret,
};
use chrono::{Duration as ChronoDuration, Utc};
use reqwest::{
    StatusCode, Url,
    blocking::{Client, multipart},
};
use serde_json::{Value, json};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

const DEFAULT_FACEBOOK_REDIRECT_URI: &str = "http://localhost/callback";
const DEFAULT_FACEBOOK_API_VERSION: &str = "v25.0";
const DEFAULT_FACEBOOK_SCOPES: &[&str] = &[
    "business_management",
    "pages_show_list",
    "read_insights",
    "pages_manage_posts",
    "pages_read_engagement",
    "pages_manage_engagement",
    "instagram_basic",
    "instagram_content_publish",
    "instagram_manage_insights",
    "instagram_manage_comments",
];

#[derive(Debug, Clone)]
pub struct FacebookUserConnection {
    pub user_id: String,
    pub user_name: String,
    pub pages: Vec<FacebookPageCandidate>,
}

#[derive(Debug, Clone)]
pub struct FacebookPageAuthorization {
    pub accounts: Vec<AccountForm>,
}

#[derive(Debug, Clone)]
pub struct FacebookPagePublishRequest {
    pub page_id: String,
    pub page_access_token: String,
    pub text: String,
    pub media_ids: Vec<String>,
    pub api_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FacebookPagePublishResponse {
    pub id: String,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct FacebookPagePhotoUploadRequest {
    pub page_id: String,
    pub page_access_token: String,
    pub file_path: PathBuf,
    pub mime_type: String,
    pub api_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FacebookPagePhotoUploadResponse {
    pub id: String,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct FacebookPageVideoUploadRequest {
    pub page_id: String,
    pub page_access_token: String,
    pub file_path: PathBuf,
    pub mime_type: String,
    pub description: String,
    pub api_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FacebookPageVideoUploadResponse {
    pub id: String,
    pub raw: Value,
}

#[derive(Debug, Clone)]
struct FacebookVideoUploadSession {
    upload_session_id: String,
    video_id: String,
    start_offset: u64,
    end_offset: u64,
    raw: Value,
}

#[derive(Debug)]
pub enum FacebookError {
    Http(reqwest::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Secret(SecretError),
    RateLimited(String),
    Unauthorized(String),
    Validation(String),
}

impl Display for FacebookError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "Facebook request failed: {error}"),
            Self::Io(error) => write!(formatter, "Facebook media file error: {error}"),
            Self::Json(error) => write!(formatter, "Facebook response was invalid: {error}"),
            Self::Secret(error) => write!(formatter, "{error}"),
            Self::RateLimited(error) => write!(formatter, "{error}"),
            Self::Unauthorized(error) => write!(formatter, "{error}"),
            Self::Validation(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for FacebookError {}

impl From<reqwest::Error> for FacebookError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

impl From<std::io::Error> for FacebookError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for FacebookError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<SecretError> for FacebookError {
    fn from(error: SecretError) -> Self {
        Self::Secret(error)
    }
}

fn normalized_facebook_api_version(version: Option<&str>) -> Result<String, FacebookError> {
    let version = version.unwrap_or(DEFAULT_FACEBOOK_API_VERSION).trim();

    if version.is_empty() {
        return Ok(DEFAULT_FACEBOOK_API_VERSION.to_string());
    }

    let valid = version
        .strip_prefix('v')
        .and_then(|value| value.split_once('.'))
        .is_some_and(|(major, minor)| {
            !major.is_empty()
                && !minor.is_empty()
                && major.chars().all(|character| character.is_ascii_digit())
                && minor.chars().all(|character| character.is_ascii_digit())
        });

    if !valid {
        return Err(FacebookError::Validation(
            "Facebook API version must look like v25.0".to_string(),
        ));
    }

    Ok(version.to_string())
}

pub fn start_facebook_oauth(
    form: &FacebookOAuthStartForm,
) -> Result<FacebookOAuthStartSummary, FacebookError> {
    let client_id = resolve_service_credential("facebook", "client_id")?;

    facebook_authorization_summary(&client_id, form)
}

pub fn exchange_facebook_oauth(
    form: &FacebookOAuthExchangeForm,
) -> Result<FacebookUserConnection, FacebookError> {
    let client_id = resolve_service_credential("facebook", "client_id")?;
    let client_secret = resolve_service_credential("facebook", "client_secret")?;
    let code = form.code.trim();
    let redirect_uri = normalized_facebook_redirect_uri(form.redirect_uri.as_deref());
    let api_version = normalized_facebook_api_version(form.api_version.as_deref())?;

    if code.is_empty() {
        return Err(FacebookError::Validation("code is required".to_string()));
    }

    let client = facebook_client()?;
    let short_token = request_facebook_access_token(
        &client,
        &client_id,
        &client_secret,
        code,
        &redirect_uri,
        &api_version,
    )?;
    let long_token = request_long_lived_facebook_access_token(
        &client,
        &client_id,
        &client_secret,
        &short_token,
        &api_version,
    )?;
    let user = fetch_facebook_user(&client, &long_token, &api_version)?;
    let user_id = required_string(&user, "id", "Facebook response missing user id")?;
    let user_name = required_string(&user, "name", "Facebook response missing user name")?;
    let pages = fetch_facebook_pages(&client, &long_token, false, &api_version)?;

    save_account_secret("facebook_user", &user_id, "access_token", &long_token)?;

    Ok(FacebookUserConnection {
        user_id,
        user_name,
        pages,
    })
}

pub fn connect_facebook_pages(
    form: &FacebookPageConnectForm,
) -> Result<FacebookPageAuthorization, FacebookError> {
    let user_id = form.user_id.trim();
    let page_ids = form
        .page_ids
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if user_id.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook user id is required".to_string(),
        ));
    }

    if page_ids.is_empty() {
        return Err(FacebookError::Validation(
            "Select at least one Facebook Page".to_string(),
        ));
    }

    let user_token = resolve_account_secret("facebook_user", user_id, "access_token")?;
    let client = facebook_client()?;
    let api_version = normalized_facebook_api_version(form.api_version.as_deref())?;
    let pages = fetch_facebook_pages(&client, &user_token, true, &api_version)?;
    let mut accounts = Vec::new();

    for page_id in page_ids {
        let page = pages
            .iter()
            .find(|candidate| candidate.id == page_id)
            .ok_or_else(|| {
                FacebookError::Validation(format!("Facebook Page {page_id} was not returned"))
            })?;
        let page_access_token = page.access_token.as_deref().ok_or_else(|| {
            FacebookError::Validation(format!("Facebook Page {page_id} missing access token"))
        })?;
        let secret_ref = save_account_secret(
            "facebook_page",
            &page.id,
            "page_access_token",
            page_access_token,
        )?;

        accounts.push(AccountForm {
            name: page.name.clone(),
            username: page.username.clone(),
            provider: "facebook_page".to_string(),
            provider_id: page.id.clone(),
            authorized: true,
            avatar_path: page.avatar_path.clone(),
            access_token_secret_ref: secret_ref,
            data: Some(json!({
                "auth": "facebook_user",
                "user_id": user_id,
                "api_version": api_version,
            })),
        });
    }

    Ok(FacebookPageAuthorization { accounts })
}

pub fn publish_facebook_page_post(
    request: &FacebookPagePublishRequest,
) -> Result<FacebookPagePublishResponse, FacebookError> {
    let page_id = request.page_id.trim();
    let page_access_token = request.page_access_token.trim();
    let text = request.text.trim();
    let media_ids = request
        .media_ids
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if page_id.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page id is required".to_string(),
        ));
    }

    if page_access_token.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page access token is required".to_string(),
        ));
    }

    if text.is_empty() && media_ids.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page post text or media is required".to_string(),
        ));
    }

    let api_version = normalized_facebook_api_version(request.api_version.as_deref())?;
    let client = facebook_client()?;
    let endpoint = format!("https://graph.facebook.com/{api_version}/{page_id}/feed");
    let mut params = vec![
        ("message".to_string(), text.to_string()),
        ("access_token".to_string(), page_access_token.to_string()),
    ];

    for (index, media_id) in media_ids.iter().enumerate() {
        params.push((
            format!("attached_media[{index}]"),
            json!({ "media_fbid": media_id }).to_string(),
        ));
    }

    let response = client.post(endpoint).form(&params).send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(facebook_http_error(
            status,
            &value,
            "Facebook Page publish failed",
        ));
    }

    facebook_publish_response(value)
}

pub fn upload_facebook_page_photo(
    request: &FacebookPagePhotoUploadRequest,
) -> Result<FacebookPagePhotoUploadResponse, FacebookError> {
    let page_id = request.page_id.trim();
    let page_access_token = request.page_access_token.trim();
    let mime_type = request.mime_type.trim();

    if page_id.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page id is required".to_string(),
        ));
    }

    if page_access_token.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page access token is required".to_string(),
        ));
    }

    if !mime_type.starts_with("image/") {
        return Err(FacebookError::Validation(format!(
            "Facebook Page photo upload does not support MIME type {mime_type}"
        )));
    }

    if !request.file_path.exists() {
        return Err(FacebookError::Validation(format!(
            "Facebook Page media file not found: {}",
            request.file_path.display()
        )));
    }

    let api_version = normalized_facebook_api_version(request.api_version.as_deref())?;
    let client = facebook_client()?;
    let endpoint = format!("https://graph.facebook.com/{api_version}/{page_id}/photos");
    let bytes = fs::read(&request.file_path)?;
    let part = multipart::Part::bytes(bytes)
        .file_name(media_file_name(&request.file_path))
        .mime_str(mime_type)
        .map_err(|error| FacebookError::Validation(format!("invalid media MIME type: {error}")))?;
    let form = multipart::Form::new()
        .part("source", part)
        .text("published", "false")
        .text("access_token", page_access_token.to_string());
    let response = client.post(endpoint).multipart(form).send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(facebook_http_error(
            status,
            &value,
            "Facebook Page photo upload failed",
        ));
    }

    facebook_photo_upload_response(value)
}

pub fn upload_facebook_page_video(
    request: &FacebookPageVideoUploadRequest,
) -> Result<FacebookPageVideoUploadResponse, FacebookError> {
    let page_id = request.page_id.trim();
    let page_access_token = request.page_access_token.trim();
    let mime_type = request.mime_type.trim();

    if page_id.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page id is required".to_string(),
        ));
    }

    if page_access_token.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page access token is required".to_string(),
        ));
    }

    if !mime_type.starts_with("video/") {
        return Err(FacebookError::Validation(format!(
            "Facebook Page video upload does not support MIME type {mime_type}"
        )));
    }

    if !request.file_path.exists() {
        return Err(FacebookError::Validation(format!(
            "Facebook Page video file not found: {}",
            request.file_path.display()
        )));
    }

    let api_version = normalized_facebook_api_version(request.api_version.as_deref())?;
    let client = facebook_client()?;
    let endpoint = format!("https://graph.facebook.com/{api_version}/{page_id}/videos");
    let file_size = fs::metadata(&request.file_path)?.len();
    let mut session =
        start_facebook_video_upload(&client, &endpoint, page_access_token, file_size)?;
    let mut file = fs::File::open(&request.file_path)?;

    while session.start_offset < session.end_offset {
        let bytes_to_read =
            usize::try_from(session.end_offset - session.start_offset).map_err(|_| {
                FacebookError::Validation("Facebook video chunk is too large".to_string())
            })?;
        let mut buffer = vec![0; bytes_to_read];

        file.seek(SeekFrom::Start(session.start_offset))?;
        file.read_exact(&mut buffer)?;

        session = transfer_facebook_video_upload(
            &client,
            &endpoint,
            page_access_token,
            &session.upload_session_id,
            &session.video_id,
            session.start_offset,
            buffer,
            media_file_name(&request.file_path),
            mime_type,
        )?;
    }

    let finish = finish_facebook_video_upload(
        &client,
        &endpoint,
        page_access_token,
        &session.upload_session_id,
        &request.description,
    )?;

    if !finish
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err(FacebookError::Validation(
            "Facebook Page video upload did not finish successfully".to_string(),
        ));
    }

    Ok(FacebookPageVideoUploadResponse {
        id: session.video_id,
        raw: json!({
            "session": session.raw,
            "finish": finish
        }),
    })
}

pub fn verify_facebook_page_account(
    page_id: &str,
    page_access_token: &str,
    access_token_secret_ref: String,
    existing_data_json: Option<&str>,
    api_version: Option<&str>,
) -> Result<AccountForm, FacebookError> {
    let api_version = normalized_facebook_api_version(api_version)?;
    let page = fetch_facebook_page(page_id, page_access_token, &api_version)?;

    facebook_page_account_form(
        &page,
        access_token_secret_ref,
        existing_data_json,
        &api_version,
    )
}

pub fn fetch_facebook_page_audience(
    page_id: &str,
    page_access_token: &str,
    api_version: Option<&str>,
) -> Result<Value, FacebookError> {
    let page_id = page_id.trim();
    let page_access_token = page_access_token.trim();

    if page_id.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page id is required".to_string(),
        ));
    }

    if page_access_token.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page access token is required".to_string(),
        ));
    }

    let api_version = normalized_facebook_api_version(api_version)?;
    let client = facebook_client()?;
    let endpoint = format!("https://graph.facebook.com/{api_version}/{page_id}");
    let response = client
        .get(endpoint)
        .query(&[
            ("fields", "fan_count,followers_count"),
            ("access_token", page_access_token),
        ])
        .send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(facebook_http_error(
            status,
            &value,
            "Facebook Page audience lookup failed",
        ));
    }

    Ok(value)
}

pub fn fetch_facebook_page_insights(
    page_id: &str,
    page_access_token: &str,
    api_version: Option<&str>,
) -> Result<Vec<Value>, FacebookError> {
    let page_id = page_id.trim();
    let page_access_token = page_access_token.trim();

    if page_id.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page id is required".to_string(),
        ));
    }

    if page_access_token.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page access token is required".to_string(),
        ));
    }

    let api_version = normalized_facebook_api_version(api_version)?;
    let client = facebook_client()?;
    let endpoint = format!("https://graph.facebook.com/{api_version}/{page_id}/insights");
    let since = (Utc::now() - ChronoDuration::days(90))
        .date_naive()
        .to_string();
    let until = Utc::now().date_naive().to_string();
    let response = client
        .get(endpoint)
        .query(&[
            ("metric", "page_post_engagements,page_posts_impressions"),
            ("period", "day"),
            ("since", since.as_str()),
            ("until", until.as_str()),
            ("access_token", page_access_token),
        ])
        .send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(facebook_http_error(
            status,
            &value,
            "Facebook Page insights lookup failed",
        ));
    }

    Ok(value
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

fn facebook_authorization_summary(
    client_id: &str,
    form: &FacebookOAuthStartForm,
) -> Result<FacebookOAuthStartSummary, FacebookError> {
    let client_id = client_id.trim();
    let api_version = normalized_facebook_api_version(form.api_version.as_deref())?;

    if client_id.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook App ID is required".to_string(),
        ));
    }

    let redirect_uri = normalized_facebook_redirect_uri(form.redirect_uri.as_deref());
    let scopes = normalized_facebook_scopes(&form.scopes);
    let state = Uuid::new_v4().to_string();
    let mut url = Url::parse(&format!(
        "https://www.facebook.com/{api_version}/dialog/oauth"
    ))
    .map_err(|error| FacebookError::Validation(format!("invalid Facebook OAuth URL: {error}")))?;

    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", &scopes.join(","))
        .append_pair("response_type", "code")
        .append_pair("state", &state);

    Ok(FacebookOAuthStartSummary {
        auth_url: url.to_string(),
        state,
        redirect_uri,
        scopes,
        api_version,
    })
}

fn request_facebook_access_token(
    client: &Client,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
    api_version: &str,
) -> Result<String, FacebookError> {
    let endpoint = format!("https://graph.facebook.com/{api_version}/oauth/access_token");
    let response = client
        .get(endpoint)
        .query(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ])
        .send()?;

    facebook_access_token_response(response, "Facebook OAuth token exchange failed")
}

fn request_long_lived_facebook_access_token(
    client: &Client,
    client_id: &str,
    client_secret: &str,
    short_token: &str,
    api_version: &str,
) -> Result<String, FacebookError> {
    let endpoint = format!("https://graph.facebook.com/{api_version}/oauth/access_token");
    let response = client
        .get(endpoint)
        .query(&[
            ("grant_type", "fb_exchange_token"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("fb_exchange_token", short_token),
        ])
        .send()?;

    facebook_access_token_response(response, "Facebook long-lived token exchange failed")
}

fn facebook_access_token_response(
    response: reqwest::blocking::Response,
    fallback: &str,
) -> Result<String, FacebookError> {
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(facebook_http_error(status, &value, fallback));
    }

    required_string(
        &value,
        "access_token",
        "Facebook response missing access_token",
    )
}

fn fetch_facebook_user(
    client: &Client,
    access_token: &str,
    api_version: &str,
) -> Result<Value, FacebookError> {
    let endpoint = format!("https://graph.facebook.com/{api_version}/me");
    let response = client
        .get(endpoint)
        .query(&[("fields", "id,name"), ("access_token", access_token)])
        .send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(facebook_http_error(
            status,
            &value,
            "Facebook user lookup failed",
        ));
    }

    Ok(value)
}

fn fetch_facebook_page(
    page_id: &str,
    page_access_token: &str,
    api_version: &str,
) -> Result<Value, FacebookError> {
    let page_id = page_id.trim();
    let page_access_token = page_access_token.trim();

    if page_id.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page id is required".to_string(),
        ));
    }

    if page_access_token.is_empty() {
        return Err(FacebookError::Validation(
            "Facebook Page access token is required".to_string(),
        ));
    }

    let client = facebook_client()?;
    let endpoint = format!("https://graph.facebook.com/{api_version}/{page_id}");
    let response = client
        .get(endpoint)
        .query(&[
            ("fields", "id,name,username,picture{url}"),
            ("access_token", page_access_token),
        ])
        .send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(facebook_http_error(
            status,
            &value,
            "Facebook Page lookup failed",
        ));
    }

    Ok(value)
}

fn fetch_facebook_pages(
    client: &Client,
    access_token: &str,
    with_access_token: bool,
    api_version: &str,
) -> Result<Vec<FacebookPageCandidate>, FacebookError> {
    let endpoint = format!("https://graph.facebook.com/{api_version}/me/accounts");
    let fields = if with_access_token {
        "id,name,username,picture{url},access_token"
    } else {
        "id,name,username,picture{url}"
    };
    let response = client
        .get(endpoint)
        .query(&[
            ("fields", fields),
            ("limit", "200"),
            ("access_token", access_token),
        ])
        .send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(facebook_http_error(
            status,
            &value,
            "Facebook Page lookup failed",
        ));
    }

    facebook_page_candidates(&value)
}

fn facebook_page_candidates(value: &Value) -> Result<Vec<FacebookPageCandidate>, FacebookError> {
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| FacebookError::Validation("Facebook response missing pages".to_string()))?;

    data.iter()
        .map(|page| {
            Ok(FacebookPageCandidate {
                id: required_string(page, "id", "Facebook Page missing id")?,
                name: required_string(page, "name", "Facebook Page missing name")?,
                username: optional_string(page, "username"),
                avatar_path: page
                    .get("picture")
                    .and_then(|picture| picture.get("data"))
                    .and_then(|data| data.get("url"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                access_token: optional_string(page, "access_token"),
            })
        })
        .collect()
}

fn facebook_page_account_form(
    page: &Value,
    access_token_secret_ref: String,
    existing_data_json: Option<&str>,
    api_version: &str,
) -> Result<AccountForm, FacebookError> {
    let provider_id = required_string(page, "id", "Facebook Page missing id")?;
    let mut metadata = existing_data_json
        .and_then(|value| serde_json::from_str::<Value>(value).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();

    metadata.insert(
        "auth".to_string(),
        Value::String("facebook_user".to_string()),
    );
    metadata.insert(
        "api_version".to_string(),
        Value::String(api_version.to_string()),
    );

    Ok(AccountForm {
        name: required_string(page, "name", "Facebook Page missing name")?,
        username: optional_string(page, "username"),
        provider: "facebook_page".to_string(),
        provider_id,
        authorized: true,
        avatar_path: page
            .get("picture")
            .and_then(|picture| picture.get("data"))
            .and_then(|data| data.get("url"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        access_token_secret_ref,
        data: Some(Value::Object(metadata)),
    })
}

fn facebook_publish_response(value: Value) -> Result<FacebookPagePublishResponse, FacebookError> {
    let id = required_string(&value, "id", "Facebook response missing post id")?;

    Ok(FacebookPagePublishResponse { id, raw: value })
}

fn facebook_photo_upload_response(
    value: Value,
) -> Result<FacebookPagePhotoUploadResponse, FacebookError> {
    let id = required_string(&value, "id", "Facebook response missing photo id")?;

    Ok(FacebookPagePhotoUploadResponse { id, raw: value })
}

fn start_facebook_video_upload(
    client: &Client,
    endpoint: &str,
    page_access_token: &str,
    file_size: u64,
) -> Result<FacebookVideoUploadSession, FacebookError> {
    let params = [
        ("upload_phase", "start".to_string()),
        ("file_size", file_size.to_string()),
        ("access_token", page_access_token.to_string()),
    ];
    let response = client.post(endpoint).form(&params).send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(facebook_http_error(
            status,
            &value,
            "Facebook Page video upload start failed",
        ));
    }

    facebook_video_upload_session(value)
}

fn transfer_facebook_video_upload(
    client: &Client,
    endpoint: &str,
    page_access_token: &str,
    upload_session_id: &str,
    video_id: &str,
    start_offset: u64,
    chunk: Vec<u8>,
    file_name: String,
    mime_type: &str,
) -> Result<FacebookVideoUploadSession, FacebookError> {
    let part = multipart::Part::bytes(chunk)
        .file_name(file_name)
        .mime_str(mime_type)
        .map_err(|error| FacebookError::Validation(format!("invalid media MIME type: {error}")))?;
    let form = multipart::Form::new()
        .text("upload_phase", "transfer")
        .text("upload_session_id", upload_session_id.to_string())
        .text("start_offset", start_offset.to_string())
        .text("access_token", page_access_token.to_string())
        .part("video_file_chunk", part);
    let response = client.post(endpoint).multipart(form).send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(facebook_http_error(
            status,
            &value,
            "Facebook Page video chunk upload failed",
        ));
    }

    facebook_video_transfer_session(value, upload_session_id, video_id)
}

fn finish_facebook_video_upload(
    client: &Client,
    endpoint: &str,
    page_access_token: &str,
    upload_session_id: &str,
    description: &str,
) -> Result<Value, FacebookError> {
    let params = [
        ("upload_phase", "finish".to_string()),
        ("upload_session_id", upload_session_id.to_string()),
        ("access_token", page_access_token.to_string()),
        ("description", description.trim().to_string()),
    ];
    let response = client.post(endpoint).form(&params).send()?;
    let status = response.status();
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(facebook_http_error(
            status,
            &value,
            "Facebook Page video upload finish failed",
        ));
    }

    Ok(value)
}

fn facebook_video_upload_session(
    value: Value,
) -> Result<FacebookVideoUploadSession, FacebookError> {
    Ok(FacebookVideoUploadSession {
        upload_session_id: required_string(
            &value,
            "upload_session_id",
            "Facebook response missing upload_session_id",
        )?,
        video_id: required_string(&value, "video_id", "Facebook response missing video_id")?,
        start_offset: required_u64_string(
            &value,
            "start_offset",
            "Facebook response missing start_offset",
        )?,
        end_offset: required_u64_string(
            &value,
            "end_offset",
            "Facebook response missing end_offset",
        )?,
        raw: value,
    })
}

fn facebook_video_transfer_session(
    value: Value,
    upload_session_id: &str,
    video_id: &str,
) -> Result<FacebookVideoUploadSession, FacebookError> {
    Ok(FacebookVideoUploadSession {
        upload_session_id: upload_session_id.to_string(),
        video_id: video_id.to_string(),
        start_offset: required_u64_string(
            &value,
            "start_offset",
            "Facebook response missing start_offset",
        )?,
        end_offset: required_u64_string(
            &value,
            "end_offset",
            "Facebook response missing end_offset",
        )?,
        raw: value,
    })
}

fn facebook_client() -> Result<Client, FacebookError> {
    Client::builder()
        .user_agent("DustWaveSocial/0.1")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(FacebookError::Http)
}

fn normalized_facebook_redirect_uri(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_FACEBOOK_REDIRECT_URI)
        .to_string()
}

fn normalized_facebook_scopes(scopes: &[String]) -> Vec<String> {
    let scopes = scopes
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if scopes.is_empty() {
        return DEFAULT_FACEBOOK_SCOPES
            .iter()
            .map(|value| value.to_string())
            .collect();
    }

    scopes
}

fn facebook_error_message(value: &Value, fallback: &str) -> String {
    value
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| value.get("error_description").and_then(Value::as_str))
        .or_else(|| value.get("message").and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn facebook_http_error(status: StatusCode, value: &Value, fallback: &str) -> FacebookError {
    let message = facebook_error_message(value, fallback);

    match status {
        StatusCode::UNAUTHORIZED => FacebookError::Unauthorized(message),
        StatusCode::TOO_MANY_REQUESTS => FacebookError::RateLimited(message),
        _ => FacebookError::Validation(message),
    }
}

fn required_string(value: &Value, key: &str, message: &str) -> Result<String, FacebookError> {
    optional_string(value, key).ok_or_else(|| FacebookError::Validation(message.to_string()))
}

fn required_u64_string(value: &Value, key: &str, message: &str) -> Result<u64, FacebookError> {
    let value = value
        .get(key)
        .ok_or_else(|| FacebookError::Validation(message.to_string()))?;

    match value {
        Value::String(value) => value
            .trim()
            .parse::<u64>()
            .map_err(|_| FacebookError::Validation(format!("{key} must be an integer"))),
        Value::Number(value) => value
            .as_u64()
            .ok_or_else(|| FacebookError::Validation(format!("{key} must be an integer"))),
        _ => Err(FacebookError::Validation(format!(
            "{key} must be an integer"
        ))),
    }
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn media_file_name(file_path: &Path) -> String {
    file_path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("media")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_facebook_authorization_url() {
        let summary = facebook_authorization_summary(
            "app-id",
            &FacebookOAuthStartForm {
                redirect_uri: Some(" http://localhost/callback ".to_string()),
                scopes: Vec::new(),
                api_version: None,
            },
        )
        .expect("summary should build");

        assert!(
            summary
                .auth_url
                .starts_with("https://www.facebook.com/v25.0/dialog/oauth?")
        );
        assert!(summary.auth_url.contains("client_id=app-id"));
        assert!(
            summary
                .auth_url
                .contains("redirect_uri=http%3A%2F%2Flocalhost%2Fcallback")
        );
        assert!(summary.auth_url.contains("response_type=code"));
        assert!(summary.auth_url.contains("pages_manage_posts"));
        assert_eq!(summary.api_version, "v25.0");
    }

    #[test]
    fn maps_facebook_page_candidates() {
        let pages = facebook_page_candidates(&serde_json::json!({
            "data": [
                {
                    "id": "page-1",
                    "name": "Dust Wave",
                    "username": "dustwave",
                    "access_token": "page-token",
                    "picture": {
                        "data": {
                            "url": "https://example.test/page.jpg"
                        }
                    }
                }
            ]
        }))
        .expect("pages should map");

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].id, "page-1");
        assert_eq!(pages[0].username.as_deref(), Some("dustwave"));
        assert_eq!(pages[0].access_token.as_deref(), Some("page-token"));
        assert_eq!(
            pages[0].avatar_path.as_deref(),
            Some("https://example.test/page.jpg")
        );
    }

    #[test]
    fn maps_facebook_http_errors_by_status() {
        assert!(matches!(
            facebook_http_error(
                StatusCode::UNAUTHORIZED,
                &serde_json::json!({ "error": { "message": "bad token" } }),
                "fallback"
            ),
            FacebookError::Unauthorized(_)
        ));
        assert!(matches!(
            facebook_http_error(
                StatusCode::TOO_MANY_REQUESTS,
                &serde_json::json!({ "error": { "message": "wait" } }),
                "fallback"
            ),
            FacebookError::RateLimited(_)
        ));
    }

    #[test]
    fn maps_facebook_publish_response() {
        let response = facebook_publish_response(serde_json::json!({
            "id": "page_123"
        }))
        .expect("publish response should map");

        assert_eq!(response.id, "page_123");
        assert_eq!(
            response.raw.get("id").and_then(Value::as_str),
            Some("page_123")
        );
    }

    #[test]
    fn maps_facebook_photo_upload_response() {
        let response = facebook_photo_upload_response(serde_json::json!({
            "id": "photo-1",
            "post_id": "page_123"
        }))
        .expect("photo response should map");

        assert_eq!(response.id, "photo-1");
    }

    #[test]
    fn rejects_facebook_publish_response_without_id() {
        let error = facebook_publish_response(serde_json::json!({}))
            .expect_err("missing post id should fail")
            .to_string();

        assert_eq!(error, "Facebook response missing post id");
    }

    #[test]
    fn maps_facebook_video_upload_session() {
        let session = facebook_video_upload_session(serde_json::json!({
            "upload_session_id": "session-1",
            "video_id": "video-1",
            "start_offset": "0",
            "end_offset": "1024"
        }))
        .expect("video session should map");

        assert_eq!(session.upload_session_id, "session-1");
        assert_eq!(session.video_id, "video-1");
        assert_eq!(session.start_offset, 0);
        assert_eq!(session.end_offset, 1024);
    }

    #[test]
    fn rejects_facebook_video_upload_session_without_offsets() {
        let error = facebook_video_upload_session(serde_json::json!({
            "upload_session_id": "session-1",
            "video_id": "video-1"
        }))
        .expect_err("missing offset should fail")
        .to_string();

        assert_eq!(error, "Facebook response missing start_offset");
    }

    #[test]
    fn maps_facebook_page_account_form() {
        let form = facebook_page_account_form(
            &serde_json::json!({
                "id": "page-1",
                "name": "Dust Wave",
                "username": "dustwave",
                "picture": {
                    "data": {
                        "url": "https://example.test/page.jpg"
                    }
                }
            }),
            "secret://accounts/facebook_page/page-1/page_access_token".to_string(),
            Some(r#"{"auth":"facebook_user","user_id":"user-1"}"#),
            "v25.0",
        )
        .expect("page account should map");

        assert_eq!(form.provider, "facebook_page");
        assert_eq!(form.provider_id, "page-1");
        assert_eq!(form.username.as_deref(), Some("dustwave"));
        assert_eq!(
            form.data
                .as_ref()
                .and_then(|data| data.get("user_id"))
                .and_then(Value::as_str),
            Some("user-1")
        );
    }
}
