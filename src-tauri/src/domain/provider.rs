use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MediaLimits {
    pub photos: u8,
    pub videos: u8,
    pub gifs: u8,
    pub allow_mixing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderPostConfig {
    pub simultaneous_posting: bool,
    pub min_text_chars: u16,
    pub max_text_chars: u16,
    pub min_media: MediaLimits,
    pub max_media: MediaLimits,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderCapability {
    pub id: &'static str,
    pub display_name: &'static str,
    pub configured_in_lite: bool,
    pub supports_oauth: bool,
    pub supports_publish: bool,
    pub supports_delete: bool,
    pub supports_audience_import: bool,
    pub supports_metrics_import: bool,
    pub requires_entity_selection: bool,
    pub post_config: ProviderPostConfig,
    pub notes: Vec<&'static str>,
}

pub fn provider_capabilities() -> Vec<ProviderCapability> {
    vec![
        ProviderCapability {
            id: "twitter",
            display_name: "X",
            configured_in_lite: true,
            supports_oauth: true,
            supports_publish: true,
            supports_delete: false,
            supports_audience_import: true,
            supports_metrics_import: true,
            requires_entity_selection: false,
            post_config: ProviderPostConfig {
                simultaneous_posting: false,
                min_text_chars: 1,
                max_text_chars: 280,
                min_media: MediaLimits {
                    photos: 1,
                    videos: 1,
                    gifs: 1,
                    allow_mixing: false,
                },
                max_media: MediaLimits {
                    photos: 4,
                    videos: 1,
                    gifs: 1,
                    allow_mixing: false,
                },
            },
            notes: vec![
                "Current Lite provider uses OAuth 1.0a and supports legacy plus v2 tweet creation paths.",
                "Identical simultaneous posting is disabled by current provider rules.",
            ],
        },
        ProviderCapability {
            id: "facebook_page",
            display_name: "Facebook Page",
            configured_in_lite: true,
            supports_oauth: true,
            supports_publish: true,
            supports_delete: false,
            supports_audience_import: true,
            supports_metrics_import: true,
            requires_entity_selection: true,
            post_config: ProviderPostConfig {
                simultaneous_posting: true,
                min_text_chars: 1,
                max_text_chars: 5000,
                min_media: MediaLimits {
                    photos: 1,
                    videos: 1,
                    gifs: 1,
                    allow_mixing: false,
                },
                max_media: MediaLimits {
                    photos: 10,
                    videos: 1,
                    gifs: 1,
                    allow_mixing: false,
                },
            },
            notes: vec![
                "Current Lite provider selects pages from a connected Facebook user account.",
                "The provider publishes images through unpublished photo attachments and videos through chunked Graph API uploads.",
            ],
        },
        ProviderCapability {
            id: "mastodon",
            display_name: "Mastodon",
            configured_in_lite: true,
            supports_oauth: true,
            supports_publish: true,
            supports_delete: false,
            supports_audience_import: true,
            supports_metrics_import: true,
            requires_entity_selection: false,
            post_config: ProviderPostConfig {
                simultaneous_posting: true,
                min_text_chars: 1,
                max_text_chars: 500,
                min_media: MediaLimits {
                    photos: 1,
                    videos: 1,
                    gifs: 1,
                    allow_mixing: false,
                },
                max_media: MediaLimits {
                    photos: 4,
                    videos: 1,
                    gifs: 1,
                    allow_mixing: false,
                },
            },
            notes: vec![
                "Current Lite provider creates a per-server app before OAuth.",
                "Media processing is asynchronous on some Mastodon servers and must be polled before posting.",
            ],
        },
        ProviderCapability {
            id: "facebook_group",
            display_name: "Facebook Group",
            configured_in_lite: false,
            supports_oauth: false,
            supports_publish: false,
            supports_delete: false,
            supports_audience_import: false,
            supports_metrics_import: false,
            requires_entity_selection: true,
            post_config: ProviderPostConfig {
                simultaneous_posting: true,
                min_text_chars: 1,
                max_text_chars: 5000,
                min_media: MediaLimits {
                    photos: 1,
                    videos: 1,
                    gifs: 1,
                    allow_mixing: false,
                },
                max_media: MediaLimits {
                    photos: 10,
                    videos: 1,
                    gifs: 1,
                    allow_mixing: false,
                },
            },
            notes: vec![
                "Existing code has UI/config traces, but no registered provider adapter.",
                "Treat this as a future Dust Wave provider, not parity scope.",
            ],
        },
    ]
}
