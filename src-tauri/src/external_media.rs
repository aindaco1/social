use crate::domain::{
    ExternalMediaItem, ExternalMediaSearchRequest, ExternalMediaSearchResult,
    ValidatedExternalMediaSearch,
};
use crate::secrets::{SecretError, resolve_service_credential};
use reqwest::blocking::Client;
use serde_json::Value;
use std::error::Error;
use std::fmt::{Display, Formatter};

const UNSPLASH_SEARCH_ENDPOINT: &str = "https://api.unsplash.com/search/photos";
const KLIPY_API_BASE: &str = "https://api.klipy.com/api/v1";
const DEFAULT_TERMS: [&str; 12] = [
    "social",
    "mix",
    "content",
    "popular",
    "viral",
    "trend",
    "light",
    "marketing",
    "self-hosted",
    "ambient",
    "writer",
    "technology",
];

#[derive(Debug)]
pub enum ExternalMediaError {
    Http(reqwest::Error),
    Json(serde_json::Error),
    Secret(SecretError),
    Validation(String),
}

impl Display for ExternalMediaError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "external media request failed: {error}"),
            Self::Json(error) => write!(formatter, "external media response was invalid: {error}"),
            Self::Secret(error) => write!(formatter, "{error}"),
            Self::Validation(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for ExternalMediaError {}

impl From<reqwest::Error> for ExternalMediaError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

impl From<serde_json::Error> for ExternalMediaError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<SecretError> for ExternalMediaError {
    fn from(error: SecretError) -> Self {
        Self::Secret(error)
    }
}

pub fn search_external_media(
    request: &ExternalMediaSearchRequest,
) -> Result<ExternalMediaSearchResult, ExternalMediaError> {
    let request = request
        .validated()
        .map_err(ExternalMediaError::Validation)?;
    let client = Client::builder()
        .user_agent("DustWaveSocial/0.1")
        .build()
        .map_err(ExternalMediaError::Http)?;

    match request.source.as_str() {
        "stock" => search_unsplash(&client, &request),
        "gifs" => search_klipy(&client, &request),
        _ => unreachable!("source is validated before searching"),
    }
}

fn search_unsplash(
    client: &Client,
    request: &ValidatedExternalMediaSearch,
) -> Result<ExternalMediaSearchResult, ExternalMediaError> {
    let client_id = resolve_service_credential("unsplash", "client_id")?;
    let keyword = search_keyword(request);
    let page = request.page.to_string();
    let per_page = request.limit.to_string();
    let text = client
        .get(UNSPLASH_SEARCH_ENDPOINT)
        .query(&[
            ("client_id", client_id.as_str()),
            ("query", keyword.as_str()),
            ("page", page.as_str()),
            ("per_page", per_page.as_str()),
        ])
        .send()?
        .error_for_status()?
        .text()?;
    let response: Value = serde_json::from_str(&text)?;

    Ok(ExternalMediaSearchResult {
        source: request.source.clone(),
        page: request.page,
        next_page: request.page + 1,
        items: unsplash_items(&response),
    })
}

fn search_klipy(
    client: &Client,
    request: &ValidatedExternalMediaSearch,
) -> Result<ExternalMediaSearchResult, ExternalMediaError> {
    let key = resolve_service_credential("klipy", "client_id")?;
    let keyword = search_keyword(request);
    let page = request.page.to_string();
    let per_page = request.limit.to_string();
    let endpoint = format!("{KLIPY_API_BASE}/{key}/gifs/search");
    let text = client
        .get(endpoint)
        .query(&[
            ("q", keyword.as_str()),
            ("page", page.as_str()),
            ("per_page", per_page.as_str()),
            ("customer_id", "dust-wave-social-desktop"),
            ("locale", "en"),
        ])
        .send()?
        .error_for_status()?
        .text()?;
    let response: Value = serde_json::from_str(&text)?;

    Ok(ExternalMediaSearchResult {
        source: request.source.clone(),
        page: request.page,
        next_page: request.page + 1,
        items: klipy_items(&response),
    })
}

fn search_keyword(request: &ValidatedExternalMediaSearch) -> String {
    request.keyword.clone().unwrap_or_else(|| {
        let index = usize::try_from(request.page - 1).unwrap_or(0) % DEFAULT_TERMS.len();
        DEFAULT_TERMS[index].to_string()
    })
}

fn unsplash_items(response: &Value) -> Vec<ExternalMediaItem> {
    response
        .get("results")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(unsplash_item).collect())
        .unwrap_or_default()
}

fn unsplash_item(item: &Value) -> Option<ExternalMediaItem> {
    let id = value_string(item, &["id"])?;
    let url = value_string(item, &["urls", "regular"])?;
    let thumb_url = value_string(item, &["urls", "thumb"]).unwrap_or_else(|| url.clone());
    let name = value_string(item, &["user", "name"])
        .or_else(|| value_string(item, &["alt_description"]))
        .unwrap_or_else(|| "Unsplash photo".to_string());
    let credit_url = value_string(item, &["user", "links", "html"]);
    let download_data = value_string(item, &["links", "download_location"])
        .map(|download_location| serde_json::json!({ "download_location": download_location }));

    Some(ExternalMediaItem {
        id,
        name,
        mime_type: "image/jpeg".to_string(),
        media_type: "image".to_string(),
        url,
        thumb_url,
        is_video: false,
        credit_url,
        download_data,
    })
}

fn klipy_items(response: &Value) -> Vec<ExternalMediaItem> {
    response
        .get("data")
        .and_then(|data| data.get("data"))
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(klipy_item).collect())
        .unwrap_or_default()
}

fn klipy_item(item: &Value) -> Option<ExternalMediaItem> {
    if value_string(item, &["type"]).as_deref() != Some("gif") {
        return None;
    }

    let id = value_string(item, &["slug"])
        .or_else(|| value_string(item, &["id"]))
        .or_else(|| klipy_file_variant_url(item, &["xs", "gif", "url"]))?;
    let url = klipy_file_variant_url(item, &["hd", "gif", "url"])
        .or_else(|| klipy_file_variant_url(item, &["md", "gif", "url"]))
        .or_else(|| klipy_file_variant_url(item, &["sm", "gif", "url"]))
        .or_else(|| klipy_file_variant_url(item, &["xs", "gif", "url"]))?;
    let thumb_url = klipy_file_variant_url(item, &["xs", "gif", "url"])
        .or_else(|| klipy_file_variant_url(item, &["sm", "gif", "url"]))
        .unwrap_or_else(|| url.clone());
    let name = value_string(item, &["title"])
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Klipy GIF".to_string());
    let download_data = serde_json::json!({
        "provider": "klipy",
        "id": value_string(item, &["id"]),
        "slug": value_string(item, &["slug"]),
    });

    Some(ExternalMediaItem {
        id,
        name,
        mime_type: "image/gif".to_string(),
        media_type: "gif".to_string(),
        url,
        thumb_url,
        is_video: false,
        credit_url: None,
        download_data: Some(download_data),
    })
}

fn klipy_file_variant_url(item: &Value, path: &[&str]) -> Option<String> {
    let mut segments = Vec::with_capacity(path.len() + 1);
    segments.push("file");
    segments.extend(path.iter().copied());
    value_string(item, &segments)
}

fn value_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;

    for segment in path {
        current = current.get(*segment)?;
    }

    current
        .as_str()
        .map(str::to_string)
        .or_else(|| current.as_i64().map(|value| value.to_string()))
        .or_else(|| current.as_u64().map(|value| value.to_string()))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_unsplash_search_results_to_mixpost_media_shape() {
        let response = serde_json::json!({
            "results": [
                {
                    "id": "abc123",
                    "urls": {
                        "regular": "https://images.unsplash.com/photo.jpg",
                        "thumb": "https://images.unsplash.com/thumb.jpg"
                    },
                    "user": {
                        "name": "Dust Photographer",
                        "links": {
                            "html": "https://unsplash.com/@dust"
                        }
                    },
                    "links": {
                        "download_location": "https://api.unsplash.com/photos/abc123/download"
                    }
                }
            ]
        });

        let items = unsplash_items(&response);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "abc123");
        assert_eq!(items[0].name, "Dust Photographer");
        assert_eq!(items[0].mime_type, "image/jpeg");
        assert_eq!(items[0].media_type, "image");
        assert_eq!(
            items[0].download_data,
            Some(serde_json::json!({
                "download_location": "https://api.unsplash.com/photos/abc123/download"
            }))
        );
    }

    #[test]
    fn maps_klipy_search_results_to_mixpost_media_shape() {
        let response = serde_json::json!({
            "result": true,
            "data": {
                "data": [
                    {
                        "id": 42,
                        "title": "Wave animation",
                        "slug": "wave-animation",
                        "type": "gif",
                        "file": {
                            "hd": {
                                "gif": {
                                    "url": "https://media.klipy.com/wave-hd.gif",
                                    "width": 480,
                                    "height": 270,
                                    "size": 200000
                                },
                                "webp": {
                                    "url": "https://media.klipy.com/wave-hd.webp",
                                    "width": 480,
                                    "height": 270,
                                    "size": 100000
                                }
                            },
                            "sm": {
                                "gif": {
                                    "url": "https://media.klipy.com/wave-sm.gif",
                                    "width": 240,
                                    "height": 135,
                                    "size": 80000
                                },
                                "webp": {
                                    "url": "https://media.klipy.com/wave-sm.webp",
                                    "width": 240,
                                    "height": 135,
                                    "size": 40000
                                }
                            }
                        }
                    }
                ],
                "current_page": 1,
                "per_page": 30,
                "has_next": true
            }
        });

        let items = klipy_items(&response);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "wave-animation");
        assert_eq!(items[0].name, "Wave animation");
        assert_eq!(items[0].mime_type, "image/gif");
        assert_eq!(items[0].media_type, "gif");
        assert_eq!(items[0].url, "https://media.klipy.com/wave-hd.gif");
        assert_eq!(items[0].thumb_url, "https://media.klipy.com/wave-sm.gif");
        assert_eq!(
            items[0].download_data,
            Some(serde_json::json!({
                "provider": "klipy",
                "id": "42",
                "slug": "wave-animation"
            }))
        );
    }
}
