use crate::secrets::{SecretError, resolve_service_credential};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MediaStagingRequest {
    pub base_url: String,
    pub file_path: PathBuf,
    pub mime_type: String,
    pub source_media_id: String,
    pub operation: String,
    pub ttl_seconds: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MediaStagingResponse {
    pub key: String,
    pub url: String,
    pub content_type: String,
    pub bytes: u64,
    pub expires_at: String,
}

#[derive(Debug)]
pub enum MediaStagingError {
    Http(reqwest::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Secret(SecretError),
    Validation(String),
}

impl Display for MediaStagingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "media staging request failed: {error}"),
            Self::Io(error) => write!(formatter, "media staging file error: {error}"),
            Self::Json(error) => write!(formatter, "media staging response was invalid: {error}"),
            Self::Secret(error) => write!(formatter, "{error}"),
            Self::Validation(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for MediaStagingError {}

impl From<reqwest::Error> for MediaStagingError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

impl From<std::io::Error> for MediaStagingError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for MediaStagingError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<SecretError> for MediaStagingError {
    fn from(error: SecretError) -> Self {
        Self::Secret(error)
    }
}

pub fn stage_media_file(
    request: &MediaStagingRequest,
) -> Result<MediaStagingResponse, MediaStagingError> {
    let base_url = request.base_url.trim().trim_end_matches('/');
    let mime_type = request.mime_type.trim();

    if !base_url.starts_with("https://") {
        return Err(MediaStagingError::Validation(
            "media staging base URL must be HTTPS".to_string(),
        ));
    }

    if !request.file_path.exists() {
        return Err(MediaStagingError::Validation(format!(
            "media staging file not found: {}",
            request.file_path.display()
        )));
    }

    if mime_type.is_empty() {
        return Err(MediaStagingError::Validation(
            "media staging MIME type is required".to_string(),
        ));
    }

    let token = resolve_service_credential("media_staging", "client_secret")?;
    let bytes = fs::read(&request.file_path)?;
    let url = format!("{base_url}/api/media/stage");
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let mut builder = client
        .post(url)
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, mime_type)
        .header("x-dustwave-source-media-id", request.source_media_id.trim())
        .header("x-dustwave-operation", request.operation.trim())
        .body(bytes);

    if let Some(ttl_seconds) = request.ttl_seconds {
        builder = builder.header("x-dustwave-ttl-seconds", ttl_seconds.to_string());
    }

    let response = builder.send()?;
    let status = response.status();
    let text = response.text()?;

    if !status.is_success() {
        let message = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| format!("media staging failed with HTTP {status}"));

        return Err(MediaStagingError::Validation(message));
    }

    let staged = serde_json::from_str::<MediaStagingResponse>(&text)?;

    if !staged.url.starts_with("https://") {
        return Err(MediaStagingError::Validation(
            "media staging response did not include an HTTPS URL".to_string(),
        ));
    }

    Ok(staged)
}
