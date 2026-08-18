use crate::domain::{AccountForm, TikTokBrokerConnectionForm};
use crate::secrets::{SecretError, save_account_secret};
use reqwest::{StatusCode, Url, blocking::Client};
use serde_json::{Value, json};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum TikTokError {
    Json(serde_json::Error),
    Reqwest(reqwest::Error),
    Secret(SecretError),
    Url(String),
    Validation(String),
}

impl Display for TikTokError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "json error: {error}"),
            Self::Reqwest(error) => write!(formatter, "request error: {error}"),
            Self::Secret(error) => write!(formatter, "{error}"),
            Self::Url(error) => write!(formatter, "url error: {error}"),
            Self::Validation(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for TikTokError {}

impl From<serde_json::Error> for TikTokError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<reqwest::Error> for TikTokError {
    fn from(error: reqwest::Error) -> Self {
        Self::Reqwest(error)
    }
}

impl From<SecretError> for TikTokError {
    fn from(error: SecretError) -> Self {
        Self::Secret(error)
    }
}

pub fn connect_tiktok_account(
    request: &TikTokBrokerConnectionForm,
) -> Result<AccountForm, TikTokError> {
    let provider_id = request.provider_id.trim();
    let name = request.name.trim();
    let broker_connection = request.broker_connection.trim();

    if provider_id.is_empty() {
        return Err(TikTokError::Validation(
            "TikTok user ID is required".to_string(),
        ));
    }

    if name.is_empty() {
        return Err(TikTokError::Validation(
            "TikTok display name is required".to_string(),
        ));
    }

    if broker_connection.is_empty() {
        return Err(TikTokError::Validation(
            "TikTok broker connection credential is required".to_string(),
        ));
    }

    let access_token_secret_ref = save_account_secret(
        "tiktok",
        provider_id,
        "broker_connection",
        broker_connection,
    )?;
    let scopes = request
        .scopes
        .iter()
        .map(|scope| scope.trim())
        .filter(|scope| !scope.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    Ok(AccountForm {
        name: name.to_string(),
        username: request
            .username
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        provider: "tiktok".to_string(),
        provider_id: provider_id.to_string(),
        authorized: true,
        avatar_path: request
            .avatar_path
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        access_token_secret_ref,
        data: Some(json!({
            "auth": "broker",
            "scopes": scopes,
        })),
    })
}

pub fn fetch_tiktok_broker_analytics(
    broker_base_url: &str,
    provider_id: &str,
    broker_connection: &str,
) -> Result<Value, TikTokError> {
    let mut url =
        Url::parse(broker_base_url.trim()).map_err(|error| TikTokError::Url(error.to_string()))?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| {
            TikTokError::Validation("TikTok broker URL cannot be a base".to_string())
        })?;
        segments
            .extend(["api", "tiktok", "accounts"])
            .push(provider_id)
            .push("analytics");
    }

    let response = Client::new()
        .get(url)
        .bearer_auth(broker_connection)
        .send()?;
    let status = response.status();
    let text = response.text()?;
    let value = serde_json::from_str::<Value>(&text)?;

    if !status.is_success() {
        return Err(tiktok_broker_error(status, &value));
    }

    Ok(value)
}

fn tiktok_broker_error(status: StatusCode, value: &Value) -> TikTokError {
    let message = value
        .get("error")
        .and_then(Value::as_str)
        .or_else(|| value.get("message").and_then(Value::as_str))
        .unwrap_or("TikTok broker request failed");

    TikTokError::Validation(format!("{message} ({status})"))
}
