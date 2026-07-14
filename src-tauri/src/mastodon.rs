use crate::domain::{
    AccountForm, MastodonAppRegistrationForm, MastodonAppRegistrationSummary, MastodonOAuthForm,
    ValidatedMastodonAppRegistration,
};
use crate::secrets::{SecretError, resolve_secret_value, save_account_secret, save_secret_value};
use reqwest::{
    StatusCode, Url,
    blocking::{
        Client,
        multipart::{Form, Part},
    },
    header::{HeaderMap, RETRY_AFTER},
};
use serde_json::{Value, json};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MastodonAccountAuthorization {
    pub server: String,
    pub account: AccountForm,
}

#[derive(Debug, Clone)]
pub struct MastodonPublishRequest {
    pub server: String,
    pub access_token: String,
    pub status: String,
    pub media_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MastodonPublishResponse {
    pub id: String,
    pub url: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct MastodonMediaUploadRequest {
    pub server: String,
    pub access_token: String,
    pub file_path: PathBuf,
    pub mime_type: String,
}

#[derive(Debug, Clone)]
pub struct MastodonMediaUploadResponse {
    pub id: String,
    pub raw: Value,
}

#[derive(Debug)]
pub enum MastodonError {
    Http(reqwest::Error),
    Json(serde_json::Error),
    Secret(SecretError),
    RateLimited {
        retry_after: Option<String>,
        message: String,
    },
    Unauthorized(String),
    Validation(String),
}

impl Display for MastodonError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "Mastodon request failed: {error}"),
            Self::Json(error) => write!(formatter, "Mastodon response was invalid: {error}"),
            Self::Secret(error) => write!(formatter, "{error}"),
            Self::RateLimited {
                retry_after: Some(retry_after),
                message,
            } => write!(formatter, "{message}; retry after {retry_after}"),
            Self::RateLimited {
                retry_after: None,
                message,
            } => write!(formatter, "{message}"),
            Self::Unauthorized(error) => write!(formatter, "{error}"),
            Self::Validation(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for MastodonError {}

impl From<reqwest::Error> for MastodonError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

impl From<serde_json::Error> for MastodonError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<SecretError> for MastodonError {
    fn from(error: SecretError) -> Self {
        Self::Secret(error)
    }
}

pub fn register_mastodon_app(
    form: &MastodonAppRegistrationForm,
) -> Result<MastodonAppRegistrationSummary, MastodonError> {
    let request = form.validated().map_err(MastodonError::Validation)?;
    let client = Client::builder()
        .user_agent("DustWaveSocial/0.1")
        .build()
        .map_err(MastodonError::Http)?;
    let endpoint = format!("https://{}/api/v1/apps", request.server);
    let mut params = vec![
        ("client_name", request.client_name.clone()),
        ("redirect_uris", request.redirect_uri.clone()),
        ("scopes", "read write".to_string()),
    ];

    if let Some(website) = request.website.as_ref() {
        params.push(("website", website.clone()));
    }

    let response = client.post(endpoint).form(&params).send()?;
    let status = response.status();
    let retry_after = retry_after_header(response.headers());
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !status.is_success() {
        return Err(mastodon_http_error(
            status,
            retry_after,
            &value,
            "Mastodon app registration failed",
        ));
    }

    mastodon_registration_summary(&request, &value)
}

pub fn connect_mastodon_account(
    form: &MastodonOAuthForm,
) -> Result<MastodonAccountAuthorization, MastodonError> {
    let request = form.validated().map_err(MastodonError::Validation)?;
    let service_name = format!("mastodon.{}", request.server);
    let client_id = resolve_secret_value(&service_name, "client_id")?;
    let client_secret = resolve_secret_value(&service_name, "client_secret")?;
    let client = Client::builder()
        .user_agent("DustWaveSocial/0.1")
        .build()
        .map_err(MastodonError::Http)?;
    let token_endpoint = format!("https://{}/oauth/token", request.server);
    let token_params = [
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("redirect_uri", request.redirect_uri.as_str()),
        ("grant_type", "authorization_code"),
        ("code", request.code.as_str()),
        ("scope", "read write"),
    ];
    let token_response = client.post(token_endpoint).form(&token_params).send()?;
    let token_status = token_response.status();
    let token_retry_after = retry_after_header(token_response.headers());
    let token_text = token_response.text()?;
    let token_value: Value = serde_json::from_str(&token_text)?;

    if !token_status.is_success() {
        return Err(mastodon_http_error(
            token_status,
            token_retry_after,
            &token_value,
            "Mastodon OAuth token exchange failed",
        ));
    }

    let access_token = mastodon_access_token(&token_value)?;
    let account_endpoint = format!(
        "https://{}/api/v1/accounts/verify_credentials",
        request.server
    );
    let account_response = client
        .get(account_endpoint)
        .bearer_auth(&access_token)
        .send()?;
    let account_status = account_response.status();
    let account_retry_after = retry_after_header(account_response.headers());
    let account_text = account_response.text()?;
    let account_value: Value = serde_json::from_str(&account_text)?;

    if !account_status.is_success() {
        return Err(mastodon_http_error(
            account_status,
            account_retry_after,
            &account_value,
            "Mastodon account verification failed",
        ));
    }

    let provider_id = mastodon_account_id(&account_value)?;
    let access_token_secret_ref =
        save_account_secret("mastodon", &provider_id, "access_token", &access_token)?;
    let account = mastodon_account_form(&request.server, &account_value, access_token_secret_ref)?;

    Ok(MastodonAccountAuthorization {
        server: request.server,
        account,
    })
}

pub fn verify_mastodon_account(
    server: &str,
    access_token: &str,
    access_token_secret_ref: String,
) -> Result<AccountForm, MastodonError> {
    let server = server.trim();
    let access_token = access_token.trim();

    if server.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon server is required".to_string(),
        ));
    }

    if access_token.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon access token is required".to_string(),
        ));
    }

    let client = Client::builder()
        .user_agent("DustWaveSocial/0.1")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(MastodonError::Http)?;

    verify_mastodon_account_with_client(&client, server, access_token, access_token_secret_ref)
}

pub fn publish_mastodon_status(
    request: &MastodonPublishRequest,
) -> Result<MastodonPublishResponse, MastodonError> {
    let server = request.server.trim();
    let access_token = request.access_token.trim();
    let status = request.status.trim();

    if server.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon server is required".to_string(),
        ));
    }

    if access_token.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon access token is required".to_string(),
        ));
    }

    if status.is_empty() && request.media_ids.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon status or media is required".to_string(),
        ));
    }

    let client = Client::builder()
        .user_agent("DustWaveSocial/0.1")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(MastodonError::Http)?;
    let endpoint = format!("https://{server}/api/v1/statuses");
    let mut params = vec![("status", status), ("visibility", "public")];

    for media_id in &request.media_ids {
        params.push(("media_ids[]", media_id.as_str()));
    }

    let response = client
        .post(endpoint)
        .bearer_auth(access_token)
        .form(&params)
        .send()?;
    let http_status = response.status();
    let retry_after = retry_after_header(response.headers());
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !http_status.is_success() {
        return Err(mastodon_http_error(
            http_status,
            retry_after,
            &value,
            "Mastodon status publish failed",
        ));
    }

    mastodon_publish_response(value)
}

pub fn upload_mastodon_media(
    request: &MastodonMediaUploadRequest,
) -> Result<MastodonMediaUploadResponse, MastodonError> {
    let server = request.server.trim();
    let access_token = request.access_token.trim();

    if server.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon server is required".to_string(),
        ));
    }

    if access_token.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon access token is required".to_string(),
        ));
    }

    if !request.file_path.exists() {
        return Err(MastodonError::Validation(format!(
            "Mastodon media file not found: {}",
            request.file_path.display()
        )));
    }

    let client = Client::builder()
        .user_agent("DustWaveSocial/0.1")
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(MastodonError::Http)?;
    let endpoint = format!("https://{server}/api/v2/media");
    let mut part = Part::file(&request.file_path)
        .map_err(|error| MastodonError::Validation(format!("media file error: {error}")))?;

    if !request.mime_type.trim().is_empty() {
        part = part.mime_str(request.mime_type.trim()).map_err(|error| {
            MastodonError::Validation(format!("invalid media MIME type: {error}"))
        })?;
    }

    if let Some(file_name) = request
        .file_path
        .file_name()
        .and_then(|value| value.to_str())
    {
        part = part.file_name(file_name.to_string());
    }

    let response = client
        .post(endpoint)
        .bearer_auth(access_token)
        .multipart(Form::new().part("file", part))
        .send()?;
    let http_status = response.status();
    let retry_after = retry_after_header(response.headers());
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !http_status.is_success() {
        return Err(mastodon_http_error(
            http_status,
            retry_after,
            &value,
            "Mastodon media upload failed",
        ));
    }

    let ready_value = wait_for_mastodon_media(&client, server, access_token, value)?;

    mastodon_media_upload_response(ready_value)
}

pub fn fetch_mastodon_account_metrics(
    server: &str,
    access_token: &str,
) -> Result<Value, MastodonError> {
    let server = server.trim();
    let access_token = access_token.trim();

    if server.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon server is required".to_string(),
        ));
    }

    if access_token.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon access token is required".to_string(),
        ));
    }

    let client = Client::builder()
        .user_agent("DustWaveSocial/0.1")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(MastodonError::Http)?;
    let endpoint = format!("https://{server}/api/v1/accounts/verify_credentials");
    let response = client.get(endpoint).bearer_auth(access_token).send()?;
    let http_status = response.status();
    let retry_after = retry_after_header(response.headers());
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !http_status.is_success() {
        return Err(mastodon_http_error(
            http_status,
            retry_after,
            &value,
            "Mastodon account metrics import failed",
        ));
    }

    Ok(value)
}

pub fn fetch_mastodon_user_statuses(
    server: &str,
    access_token: &str,
    user_id: &str,
) -> Result<Vec<Value>, MastodonError> {
    let server = server.trim();
    let access_token = access_token.trim();
    let user_id = user_id.trim();

    if server.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon server is required".to_string(),
        ));
    }

    if access_token.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon access token is required".to_string(),
        ));
    }

    if user_id.is_empty() {
        return Err(MastodonError::Validation(
            "Mastodon user id is required".to_string(),
        ));
    }

    let client = Client::builder()
        .user_agent("DustWaveSocial/0.1")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(MastodonError::Http)?;
    let endpoint = format!("https://{server}/api/v1/accounts/{user_id}/statuses");
    let response = client
        .get(endpoint)
        .bearer_auth(access_token)
        .query(&[
            ("exclude_replies", "true"),
            ("exclude_reblogs", "true"),
            ("limit", "40"),
        ])
        .send()?;
    let http_status = response.status();
    let retry_after = retry_after_header(response.headers());
    let text = response.text()?;
    let value: Value = serde_json::from_str(&text)?;

    if !http_status.is_success() {
        return Err(mastodon_http_error(
            http_status,
            retry_after,
            &value,
            "Mastodon statuses import failed",
        ));
    }

    match value {
        Value::Array(items) => Ok(items),
        _ => Err(MastodonError::Validation(
            "Mastodon statuses response was not a list".to_string(),
        )),
    }
}

fn mastodon_registration_summary(
    request: &ValidatedMastodonAppRegistration,
    value: &Value,
) -> Result<MastodonAppRegistrationSummary, MastodonError> {
    if let Some(error) = value.get("error").and_then(Value::as_str) {
        return Err(MastodonError::Validation(error.to_string()));
    }

    let client_id = value
        .get("client_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            MastodonError::Validation("Mastodon response missing client_id".to_string())
        })?;
    let client_secret = value
        .get("client_secret")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            MastodonError::Validation("Mastodon response missing client_secret".to_string())
        })?;
    let service_name = format!("mastodon.{}", request.server);
    let client_id_ref = save_secret_value(&service_name, "client_id", client_id)?;
    let client_secret_ref = save_secret_value(&service_name, "client_secret", client_secret)?;
    let auth_url = mastodon_auth_url(&request.server, client_id, &request.redirect_uri)?;

    Ok(MastodonAppRegistrationSummary {
        server: request.server.clone(),
        service_name,
        client_id_ref,
        client_secret_ref,
        auth_url,
    })
}

fn mastodon_auth_url(
    server: &str,
    client_id: &str,
    redirect_uri: &str,
) -> Result<String, MastodonError> {
    let mut url = Url::parse(&format!("https://{server}/oauth/authorize"))
        .map_err(|error| MastodonError::Validation(format!("invalid Mastodon server: {error}")))?;

    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", "read write")
        .append_pair("response_type", "code");

    Ok(url.to_string())
}

fn verify_mastodon_account_with_client(
    client: &Client,
    server: &str,
    access_token: &str,
    access_token_secret_ref: String,
) -> Result<AccountForm, MastodonError> {
    let account_endpoint = format!("https://{server}/api/v1/accounts/verify_credentials");
    let account_response = client
        .get(account_endpoint)
        .bearer_auth(access_token)
        .send()?;
    let account_status = account_response.status();
    let account_retry_after = retry_after_header(account_response.headers());
    let account_text = account_response.text()?;
    let account_value: Value = serde_json::from_str(&account_text)?;

    if !account_status.is_success() {
        return Err(mastodon_http_error(
            account_status,
            account_retry_after,
            &account_value,
            "Mastodon account verification failed",
        ));
    }

    mastodon_account_form(server, &account_value, access_token_secret_ref)
}

fn mastodon_error_message(value: &Value, fallback: &str) -> String {
    value
        .get("error_description")
        .and_then(Value::as_str)
        .or_else(|| value.get("error").and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn mastodon_http_error(
    status: StatusCode,
    retry_after: Option<String>,
    value: &Value,
    fallback: &str,
) -> MastodonError {
    let message = mastodon_error_message(value, fallback);

    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return MastodonError::Unauthorized(message);
    }

    if status == StatusCode::TOO_MANY_REQUESTS {
        return MastodonError::RateLimited {
            retry_after,
            message,
        };
    }

    MastodonError::Validation(message)
}

fn retry_after_header(headers: &HeaderMap) -> Option<String> {
    headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn mastodon_access_token(value: &Value) -> Result<String, MastodonError> {
    value
        .get("access_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            MastodonError::Validation("Mastodon response missing access_token".to_string())
        })
}

fn mastodon_account_id(value: &Value) -> Result<String, MastodonError> {
    json_string(value, "id").ok_or_else(|| {
        MastodonError::Validation("Mastodon response missing account id".to_string())
    })
}

fn mastodon_account_form(
    server: &str,
    value: &Value,
    access_token_secret_ref: String,
) -> Result<AccountForm, MastodonError> {
    let provider_id = mastodon_account_id(value)?;
    let username =
        trimmed_json_string(value, "acct").or_else(|| trimmed_json_string(value, "username"));
    let name = trimmed_json_string(value, "display_name")
        .or_else(|| username.clone())
        .unwrap_or_else(|| provider_id.clone());
    let avatar_path = trimmed_json_string(value, "avatar");
    let profile_url = trimmed_json_string(value, "url");

    Ok(AccountForm {
        name,
        username,
        provider: "mastodon".to_string(),
        provider_id,
        authorized: true,
        avatar_path,
        access_token_secret_ref,
        data: Some(json!({
            "server": server,
            "profile_url": profile_url,
        })),
    })
}

fn mastodon_publish_response(value: Value) -> Result<MastodonPublishResponse, MastodonError> {
    let id = json_string(&value, "id").ok_or_else(|| {
        MastodonError::Validation("Mastodon response missing status id".to_string())
    })?;
    let url = trimmed_json_string(&value, "url");

    Ok(MastodonPublishResponse {
        id,
        url,
        raw: value,
    })
}

fn mastodon_media_upload_response(
    value: Value,
) -> Result<MastodonMediaUploadResponse, MastodonError> {
    let id = json_string(&value, "id").ok_or_else(|| {
        MastodonError::Validation("Mastodon response missing media id".to_string())
    })?;

    Ok(MastodonMediaUploadResponse { id, raw: value })
}

fn wait_for_mastodon_media(
    client: &Client,
    server: &str,
    access_token: &str,
    initial: Value,
) -> Result<Value, MastodonError> {
    let id = json_string(&initial, "id").ok_or_else(|| {
        MastodonError::Validation("Mastodon response missing media id".to_string())
    })?;

    if mastodon_media_ready(&initial) {
        return Ok(initial);
    }

    let endpoint = format!("https://{server}/api/v1/media/{id}");

    for _ in 0..15 {
        thread::sleep(Duration::from_secs(2));

        let response = client.get(&endpoint).bearer_auth(access_token).send()?;
        let http_status = response.status();
        let retry_after = retry_after_header(response.headers());
        let text = response.text()?;
        let value: Value = serde_json::from_str(&text)?;

        if !http_status.is_success() {
            return Err(mastodon_http_error(
                http_status,
                retry_after,
                &value,
                "Mastodon media processing check failed",
            ));
        }

        if mastodon_media_ready(&value) {
            return Ok(value);
        }
    }

    Err(MastodonError::Validation(
        "Mastodon media upload is still processing".to_string(),
    ))
}

fn mastodon_media_ready(value: &Value) -> bool {
    ["url", "preview_url", "remote_url", "text_url"]
        .iter()
        .any(|key| {
            value
                .get(*key)
                .and_then(Value::as_str)
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
        })
}

fn trimmed_json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    match value.get(key)? {
        Value::String(value) => {
            let trimmed = value.trim();

            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_mastodon_authorization_url() {
        let url = mastodon_auth_url("mastodon.social", "client-id", "urn:ietf:wg:oauth:2.0:oob")
            .expect("auth url should build");

        assert!(url.starts_with("https://mastodon.social/oauth/authorize?"));
        assert!(url.contains("client_id=client-id"));
        assert!(url.contains("redirect_uri=urn%3Aietf%3Awg%3Aoauth%3A2.0%3Aoob"));
        assert!(url.contains("scope=read+write"));
        assert!(url.contains("response_type=code"));
    }

    #[test]
    fn rejects_registration_responses_without_credentials() {
        let request = MastodonAppRegistrationForm {
            server: "https://mastodon.social".to_string(),
            client_name: None,
            redirect_uri: None,
            website: None,
        }
        .validated()
        .expect("registration request should validate");
        let error = mastodon_registration_summary(&request, &serde_json::json!({ "id": "1" }))
            .expect_err("missing credentials should fail")
            .to_string();

        assert_eq!(error, "Mastodon response missing client_id");
    }

    #[test]
    fn rejects_oauth_token_responses_without_access_token() {
        let error = mastodon_access_token(&serde_json::json!({ "token_type": "Bearer" }))
            .expect_err("missing access token should fail")
            .to_string();

        assert_eq!(error, "Mastodon response missing access_token");
    }

    #[test]
    fn maps_verified_account_to_account_form() {
        let account = mastodon_account_form(
            "mastodon.social",
            &serde_json::json!({
                "id": "109973234",
                "username": "dustwave",
                "acct": "dustwave",
                "display_name": "Dust Wave",
                "avatar": "https://mastodon.social/avatar.png",
                "url": "https://mastodon.social/@dustwave"
            }),
            "secret://accounts/mastodon/109973234/access_token".to_string(),
        )
        .expect("account form should map");

        assert_eq!(account.name, "Dust Wave");
        assert_eq!(account.username.as_deref(), Some("dustwave"));
        assert_eq!(account.provider, "mastodon");
        assert_eq!(account.provider_id, "109973234");
        assert_eq!(
            account.access_token_secret_ref,
            "secret://accounts/mastodon/109973234/access_token"
        );
        assert_eq!(
            account
                .data
                .as_ref()
                .and_then(|value| value.get("server"))
                .and_then(Value::as_str),
            Some("mastodon.social")
        );
    }

    #[test]
    fn maps_publish_response() {
        let response = mastodon_publish_response(serde_json::json!({
            "id": "111",
            "url": "https://mastodon.social/@dustwave/111",
            "content": "Launch"
        }))
        .expect("publish response should map");

        assert_eq!(response.id, "111");
        assert_eq!(
            response.url.as_deref(),
            Some("https://mastodon.social/@dustwave/111")
        );
        assert_eq!(
            response.raw.get("content").and_then(Value::as_str),
            Some("Launch")
        );
    }

    #[test]
    fn rejects_publish_responses_without_status_id() {
        let error = mastodon_publish_response(serde_json::json!({ "url": "https://example.test" }))
            .expect_err("missing status id should fail")
            .to_string();

        assert_eq!(error, "Mastodon response missing status id");
    }

    #[test]
    fn maps_media_upload_response() {
        let response = mastodon_media_upload_response(serde_json::json!({
            "id": "media-111",
            "url": "https://mastodon.social/media/file.png"
        }))
        .expect("media response should map");

        assert_eq!(response.id, "media-111");
        assert!(mastodon_media_ready(&response.raw));
    }

    #[test]
    fn rejects_media_upload_responses_without_media_id() {
        let error =
            mastodon_media_upload_response(serde_json::json!({ "url": "https://example.test" }))
                .expect_err("missing media id should fail")
                .to_string();

        assert_eq!(error, "Mastodon response missing media id");
    }

    #[test]
    fn maps_unauthorized_http_errors() {
        let error = mastodon_http_error(
            StatusCode::UNAUTHORIZED,
            None,
            &serde_json::json!({ "error": "The access token is invalid" }),
            "fallback",
        );

        assert!(matches!(error, MastodonError::Unauthorized(_)));
        assert_eq!(error.to_string(), "The access token is invalid");
    }

    #[test]
    fn maps_rate_limit_http_errors_with_retry_after() {
        let error = mastodon_http_error(
            StatusCode::TOO_MANY_REQUESTS,
            Some("60".to_string()),
            &serde_json::json!({ "error": "Too many requests" }),
            "fallback",
        );

        assert!(matches!(error, MastodonError::RateLimited { .. }));
        assert_eq!(error.to_string(), "Too many requests; retry after 60");
    }
}
