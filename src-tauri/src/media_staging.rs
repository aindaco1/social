use crate::secrets::{SecretError, resolve_service_credential};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize)]
struct MediaStagingEnrollmentRequest<'a> {
    enrollment_code: &'a str,
}

#[derive(Debug, Deserialize)]
struct MediaStagingEnrollmentResponse {
    token: String,
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

    validate_base_url(base_url)?;

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

    validate_response(status, &text, "media staging")?;

    let staged = serde_json::from_str::<MediaStagingResponse>(&text)?;

    if !staged.url.starts_with("https://") {
        return Err(MediaStagingError::Validation(
            "media staging response did not include an HTTPS URL".to_string(),
        ));
    }

    Ok(staged)
}

pub fn enroll_media_staging_device(
    base_url: &str,
    enrollment_code: &str,
) -> Result<String, MediaStagingError> {
    let base_url = base_url.trim().trim_end_matches('/');
    let enrollment_code = enrollment_code.trim();

    validate_base_url(base_url)?;

    if enrollment_code.is_empty() {
        return Err(MediaStagingError::Validation(
            "one-time setup code is required".to_string(),
        ));
    }

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    let response = client
        .post(format!("{base_url}/api/enroll"))
        .json(&MediaStagingEnrollmentRequest { enrollment_code })
        .send()?;
    let status = response.status();
    let text = response.text()?;

    validate_response(status, &text, "device pairing")?;
    let enrollment = serde_json::from_str::<MediaStagingEnrollmentResponse>(&text)?;
    let token = enrollment.token.trim();

    if token.is_empty() {
        return Err(MediaStagingError::Validation(
            "device pairing did not return an access token".to_string(),
        ));
    }

    Ok(token.to_string())
}

fn validate_base_url(base_url: &str) -> Result<(), MediaStagingError> {
    if !base_url.starts_with("https://") {
        return Err(MediaStagingError::Validation(
            "Instagram local media service URL must use HTTPS".to_string(),
        ));
    }

    Ok(())
}

fn validate_response(
    status: reqwest::StatusCode,
    text: &str,
    operation: &str,
) -> Result<(), MediaStagingError> {
    if status.is_success() {
        return Ok(());
    }

    let error = serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| format!("{operation} failed with HTTP {status}"));
    let message = match error.as_str() {
        "invalid_enrollment_code" => "That setup code is not valid.".to_string(),
        "enrollment_code_not_found" => {
            "That setup code was already used or is not recognized. Ask for a new code.".to_string()
        }
        "enrollment_code_expired" => {
            "That setup code expired. Ask for a new code and try again.".to_string()
        }
        _ => error,
    };

    Err(MediaStagingError::Validation(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrollment_requires_https_and_a_code_before_network_access() {
        assert!(
            enroll_media_staging_device("http://media.example", "code")
                .unwrap_err()
                .to_string()
                .contains("must use HTTPS")
        );
        assert!(
            enroll_media_staging_device("https://media.example", "")
                .unwrap_err()
                .to_string()
                .contains("setup code is required")
        );
    }

    #[test]
    fn enrollment_errors_are_written_for_people() {
        let error = validate_response(
            reqwest::StatusCode::NOT_FOUND,
            r#"{"error":"enrollment_code_not_found"}"#,
            "device pairing",
        )
        .unwrap_err();

        assert!(error.to_string().contains("already used"));
    }
}
