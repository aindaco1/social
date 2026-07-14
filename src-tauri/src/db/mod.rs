use crate::domain::{
    AccountForm, AccountImportQueueBatchSummary, AccountSummary, AppSettings, AudienceForm,
    AudiencePoint, AudienceReport, BulkDeletePostsForm, BulkDeletePostsSummary,
    DashboardAccountCounts, DashboardJobCounts, DashboardPostCounts, DashboardProviderSummary,
    DashboardSummary, ExternalMediaItem, FacebookImportSummary, FacebookInsightForm, JobForm,
    JobSummary, LocalBackupExportSummary, LocalBackupRestoreForm, LocalBackupRestoreSummary,
    LocalDataSnapshot, MastodonImportSummary, MediaCleanupSummary, MediaDownloadForm,
    MediaImportForm, MediaLibraryItem, MediaLibraryRequest, MediaSummary, MetricForm,
    PostCalendarWindow, PostContentBlock, PostDetail, PostForm, PostListAccount, PostListTag,
    PostQueryRequest, PostQueryResult, PostSummary, PostValidationError, PostValidationReport,
    PostVersionForm, ProviderCapability, RateLimitSummary, ReportMetric, ReportRequest,
    ReportSnapshot, SchedulePostForm, ServiceForm, ServiceSummary, StaleJobRecoverySummary,
    SystemHealthCounts, SystemLogClearSummary, SystemLogExportSummary, SystemLogFile,
    SystemMaintenanceSummary, TagForm, TagSummary, TwitterImportSummary,
    ValidatedMediaLibraryQuery, ValidatedPostQuery, WorkerJobOutcome, WorkerRunSummary,
    media_conversion_count, post_preview, post_schedule_status_label, post_status_label,
    provider_capabilities,
};
#[cfg(test)]
use crate::domain::{MediaForm, RateLimitForm};
use crate::facebook::{
    FacebookError, FacebookPagePhotoUploadRequest, FacebookPagePublishRequest,
    FacebookPageVideoUploadRequest, fetch_facebook_page_audience, fetch_facebook_page_insights,
    publish_facebook_page_post, upload_facebook_page_photo, upload_facebook_page_video,
    verify_facebook_page_account,
};
use crate::mastodon::{
    MastodonError, MastodonMediaUploadRequest, MastodonPublishRequest,
    fetch_mastodon_account_metrics, fetch_mastodon_user_statuses, publish_mastodon_status,
    upload_mastodon_media, verify_mastodon_account,
};
use crate::media_tools::media_tool_command;
use crate::secrets::resolve_account_secret;
use crate::twitter::{
    TwitterError, TwitterMediaUploadRequest, TwitterPublishRequest, fetch_twitter_account_metrics,
    fetch_twitter_user_posts, publish_twitter_post, upload_twitter_media, verify_twitter_account,
};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use image::imageops::FilterType;
use reqwest::{Url, blocking::Client, header::CONTENT_TYPE, redirect::Policy};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, Transaction, params, params_from_iter};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

const INITIAL_SCHEMA_VERSION: u16 = 1;
const SYSTEM_LOGS_SCHEMA_VERSION: u16 = 2;
const JOB_IDEMPOTENCY_SCHEMA_VERSION: u16 = 3;
const SERVICE_CONFIGURATION_SCHEMA_VERSION: u16 = 4;
const CURRENT_SCHEMA_VERSION: u16 = SERVICE_CONFIGURATION_SCHEMA_VERSION;
const INITIAL_SCHEMA_FILENAME: &str = "0001_initial.sql";
const INITIAL_SCHEMA_SQL: &str = include_str!("../../migrations/0001_initial.sql");
const SYSTEM_LOGS_SCHEMA_FILENAME: &str = "0002_system_logs.sql";
const SYSTEM_LOGS_SCHEMA_SQL: &str = include_str!("../../migrations/0002_system_logs.sql");
const JOB_IDEMPOTENCY_SCHEMA_FILENAME: &str = "0003_job_idempotency.sql";
const JOB_IDEMPOTENCY_SCHEMA_SQL: &str = include_str!("../../migrations/0003_job_idempotency.sql");
const SERVICE_CONFIGURATION_SCHEMA_FILENAME: &str = "0004_service_configuration.sql";
const SERVICE_CONFIGURATION_SCHEMA_SQL: &str =
    include_str!("../../migrations/0004_service_configuration.sql");
const SCHEMA_MIGRATIONS: &[SchemaMigration] = &[
    SchemaMigration {
        version: INITIAL_SCHEMA_VERSION,
        filename: INITIAL_SCHEMA_FILENAME,
        sql: INITIAL_SCHEMA_SQL,
    },
    SchemaMigration {
        version: SYSTEM_LOGS_SCHEMA_VERSION,
        filename: SYSTEM_LOGS_SCHEMA_FILENAME,
        sql: SYSTEM_LOGS_SCHEMA_SQL,
    },
    SchemaMigration {
        version: JOB_IDEMPOTENCY_SCHEMA_VERSION,
        filename: JOB_IDEMPOTENCY_SCHEMA_FILENAME,
        sql: JOB_IDEMPOTENCY_SCHEMA_SQL,
    },
    SchemaMigration {
        version: SERVICE_CONFIGURATION_SCHEMA_VERSION,
        filename: SERVICE_CONFIGURATION_SCHEMA_FILENAME,
        sql: SERVICE_CONFIGURATION_SCHEMA_SQL,
    },
];
const MAX_IMAGE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_GIF_BYTES: u64 = 15 * 1024 * 1024;
const MAX_VIDEO_BYTES: u64 = 200 * 1024 * 1024;
const SYSTEM_LOG_NAME: &str = "dust-wave-system.log";
const SYSTEM_LOG_PREVIEW_BYTES: usize = 3 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Database {
    path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
struct SchemaMigration {
    version: u16,
    filename: &'static str,
    sql: &'static str,
}

#[derive(Debug, Clone)]
struct RateLimitBlock {
    scope: String,
    retry_after_at: String,
}

#[derive(Debug, Clone)]
struct PublishTarget {
    post_account_id: i64,
    account_id: i64,
    provider: String,
    provider_id: String,
    account_data_json: Option<String>,
    content: PostContentBlock,
}

#[derive(Debug)]
struct PublishAccountResult {
    post_account_id: i64,
    provider_post_id: Option<String>,
    data_json: Option<String>,
    errors_json: Option<String>,
    error_detail: Option<String>,
    published_remotely: bool,
}

#[derive(Debug)]
struct PublishMediaAsset {
    id: String,
    file_path: PathBuf,
    mime_type: String,
    temporary: bool,
}

impl PublishMediaAsset {
    fn cleanup(&self) {
        if self.temporary {
            let _ = fs::remove_file(&self.file_path);
        }
    }
}

fn cleanup_publish_assets(assets: &[PublishMediaAsset]) {
    for asset in assets {
        asset.cleanup();
    }
}

#[derive(Debug, Default)]
struct PublishBatchResult {
    results: Vec<PublishAccountResult>,
    failures: Vec<String>,
    remote_count: usize,
}

#[derive(Debug, Default)]
struct MediaKindCounts {
    photos: usize,
    videos: usize,
    gifs: usize,
    missing: usize,
}

#[derive(Debug)]
struct StoredMediaFile {
    original_name: String,
    mime_type: String,
    relative_path: String,
    absolute_path: PathBuf,
    size: i64,
}

#[derive(Debug)]
struct MediaConversionArtifacts {
    conversions_json: String,
    size_total: i64,
}

#[derive(Debug)]
struct GeneratedConversion {
    engine: &'static str,
    path: String,
    size: i64,
}

impl MediaKindCounts {
    fn mixed(&self) -> bool {
        [self.photos, self.videos, self.gifs]
            .into_iter()
            .filter(|count| *count > 0)
            .count()
            > 1
    }
}

#[derive(Debug)]
pub enum DbError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Path(tauri::Error),
    Sqlite(rusqlite::Error),
    Validation(String),
}

impl Display for DbError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "filesystem error: {error}"),
            Self::Json(error) => write!(formatter, "json error: {error}"),
            Self::Path(error) => write!(formatter, "app data path error: {error}"),
            Self::Sqlite(error) => write!(formatter, "database error: {error}"),
            Self::Validation(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for DbError {}

impl From<std::io::Error> for DbError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<serde_json::Error> for DbError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<tauri::Error> for DbError {
    fn from(error: tauri::Error) -> Self {
        Self::Path(error)
    }
}

impl Database {
    pub fn initialize(app: &AppHandle) -> Result<Self, DbError> {
        let directory = app.path().app_data_dir()?;
        fs::create_dir_all(&directory)?;

        Self::initialize_at(directory.join("dust-wave-social.sqlite3"))
    }

    pub fn initialize_at(path: impl Into<PathBuf>) -> Result<Self, DbError> {
        let database = Self { path: path.into() };
        database.migrate()?;

        Ok(database)
    }

    fn media_storage_directory(&self) -> PathBuf {
        self.path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("media")
    }

    fn managed_media_file_paths(
        &self,
        disk: &str,
        path: &str,
        conversions_json: Option<&str>,
    ) -> Vec<PathBuf> {
        let media_directory = self.media_storage_directory();
        let mut paths = Vec::new();

        if disk == "local" {
            if let Some(suffix) = managed_media_file_suffix(path) {
                paths.push(media_directory.join(suffix));
            }
        }

        for suffix in managed_media_conversion_suffixes(conversions_json) {
            paths.push(media_directory.join(suffix));
        }

        paths
    }

    fn media_resource_path(&self, disk: &str, path: &str) -> Option<String> {
        if disk == "local" {
            return managed_media_file_suffix(path)
                .map(|suffix| self.media_storage_directory().join(suffix))
                .map(|path| path.display().to_string());
        }

        if path.starts_with("http://") || path.starts_with("https://") {
            return Some(path.to_string());
        }

        None
    }

    fn media_conversion_resource_path(
        &self,
        conversions_json: Option<&str>,
        conversion_name: &str,
    ) -> Option<String> {
        let conversion = media_conversion_entry(conversions_json, conversion_name)?;
        let path = conversion.get("path")?.as_str()?;
        let disk = conversion
            .get("disk")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("local");

        self.media_resource_path(disk, path)
    }

    fn store_downloaded_file_url(&self, uuid: &str, url: &Url) -> Result<StoredMediaFile, DbError> {
        let source_path = url
            .to_file_path()
            .map_err(|_| DbError::Validation("file URL must point to a local path".to_string()))?;
        let metadata = fs::metadata(&source_path)?;

        if !metadata.is_file() {
            return Err(DbError::Validation(
                "file URL must point to a file".to_string(),
            ));
        }

        let original_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("downloaded-media")
            .to_string();
        let mime_type = infer_media_mime_type(&source_path).to_string();
        validate_media_upload(&mime_type, metadata.len())?;

        let filename = media_storage_filename(uuid, &source_path);
        let media_directory = self.media_storage_directory();
        let destination_path = media_directory.join(&filename);

        fs::create_dir_all(&media_directory)?;
        fs::copy(&source_path, &destination_path)?;

        Ok(StoredMediaFile {
            original_name,
            mime_type,
            relative_path: format!("media/{filename}"),
            absolute_path: destination_path,
            size: i64::try_from(metadata.len()).map_err(|_| {
                DbError::Validation("downloaded media is too large to track".to_string())
            })?,
        })
    }

    fn store_downloaded_http_url(&self, uuid: &str, url: &Url) -> Result<StoredMediaFile, DbError> {
        let client = Client::builder()
            .redirect(Policy::limited(5))
            .user_agent("Dust Wave Social/0.1")
            .build()
            .map_err(|error| DbError::Validation(format!("download client failed: {error}")))?;
        let response = client
            .get(url.clone())
            .send()
            .map_err(|error| DbError::Validation(format!("download failed: {error}")))?;

        if !response.status().is_success() {
            return Err(DbError::Validation(format!(
                "download failed with HTTP {}",
                response.status()
            )));
        }

        let header_mime_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(trim_content_type)
            .filter(|value| !value.is_empty());
        let original_name = url_file_name(url).unwrap_or_else(|| "downloaded-media".to_string());
        let fallback_mime_type = infer_media_mime_type(Path::new(&original_name)).to_string();
        let mime_type = header_mime_type.unwrap_or(fallback_mime_type);

        if let Some(length) = response.content_length() {
            validate_media_upload(&mime_type, length)?;
        }

        let extension = Path::new(&original_name)
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_string)
            .or_else(|| extension_for_mime_type(&mime_type).map(str::to_string));
        let filename = media_storage_filename_from_extension(uuid, extension.as_deref());
        let media_directory = self.media_storage_directory();
        let destination_path = media_directory.join(&filename);

        fs::create_dir_all(&media_directory)?;

        let mut destination = fs::File::create(&destination_path)?;
        let mut limited = response.take(MAX_VIDEO_BYTES + 1);
        let size = std::io::copy(&mut limited, &mut destination)?;

        if let Err(error) = validate_media_upload(&mime_type, size) {
            let _ = fs::remove_file(&destination_path);
            return Err(error);
        }

        Ok(StoredMediaFile {
            original_name,
            mime_type,
            relative_path: format!("media/{filename}"),
            absolute_path: destination_path,
            size: i64::try_from(size).map_err(|_| {
                DbError::Validation("downloaded media is too large to track".to_string())
            })?,
        })
    }

    fn transient_external_media_asset(
        &self,
        media: &ExternalMediaItem,
    ) -> Result<PublishMediaAsset, DbError> {
        media.validated_reference().map_err(DbError::Validation)?;
        let url = Url::parse(media.url.trim())
            .map_err(|error| DbError::Validation(format!("invalid external media url: {error}")))?;

        if !matches!(url.scheme(), "http" | "https") {
            return Err(DbError::Validation(
                "external media url must start with http:// or https://".to_string(),
            ));
        }

        let client = Client::builder()
            .redirect(Policy::limited(5))
            .user_agent("Dust Wave Social/0.1")
            .build()
            .map_err(|error| {
                DbError::Validation(format!("external media client failed: {error}"))
            })?;
        let response = client.get(url.clone()).send().map_err(|error| {
            DbError::Validation(format!("external media fetch failed: {error}"))
        })?;

        if !response.status().is_success() {
            return Err(DbError::Validation(format!(
                "external media fetch failed with HTTP {}",
                response.status()
            )));
        }

        let declared_mime_type = media.mime_type.trim().to_string();
        let header_mime_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(trim_content_type)
            .filter(|value| extension_for_mime_type(value).is_some());

        if header_mime_type
            .as_deref()
            .is_some_and(|mime_type| mime_type != declared_mime_type)
        {
            return Err(DbError::Validation(format!(
                "external media response MIME type must match {declared_mime_type}"
            )));
        }

        let mime_type = declared_mime_type;

        if let Some(length) = response.content_length() {
            validate_media_upload(&mime_type, length)?;
        }

        let original_name = url_file_name(&url)
            .or_else(|| Some(media.name.trim().to_string()))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "klipy.gif".to_string());
        let extension = Path::new(&original_name)
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_string)
            .or_else(|| extension_for_mime_type(&mime_type).map(str::to_string));
        let directory = std::env::temp_dir().join("dust-wave-social-publish");
        let filename = media_storage_filename_from_extension(
            &format!("klipy-{}", Uuid::new_v4()),
            extension.as_deref(),
        );
        let file_path = directory.join(filename);

        fs::create_dir_all(&directory)?;

        let mut destination = fs::File::create(&file_path)?;
        let mut limited = response.take(MAX_VIDEO_BYTES + 1);
        let size = std::io::copy(&mut limited, &mut destination)?;

        if let Err(error) = validate_media_upload(&mime_type, size) {
            let _ = fs::remove_file(&file_path);
            return Err(error);
        }

        Ok(PublishMediaAsset {
            id: format!("klipy:{}", media.id.trim()),
            file_path,
            mime_type,
            temporary: true,
        })
    }

    fn media_conversion_artifacts(
        &self,
        uuid: &str,
        stored: &StoredMediaFile,
    ) -> Result<MediaConversionArtifacts, DbError> {
        let mut size_total = stored.size;
        let mut conversions = Vec::new();

        let conversion = match self.create_image_thumb_conversion(uuid, stored)? {
            Some(conversion) => Some(conversion),
            None => self.create_video_thumb_conversion(uuid, stored)?,
        };

        if let Some(conversion) = conversion {
            size_total += conversion.size;
            conversions.push(serde_json::json!({
                "engine": conversion.engine,
                "path": conversion.path,
                "disk": "local",
                "name": "thumb",
            }));
        }

        Ok(MediaConversionArtifacts {
            conversions_json: serde_json::to_string(&conversions)?,
            size_total,
        })
    }

    fn create_image_thumb_conversion(
        &self,
        uuid: &str,
        stored: &StoredMediaFile,
    ) -> Result<Option<GeneratedConversion>, DbError> {
        if !stored.mime_type.starts_with("image/") || stored.mime_type == "image/gif" {
            return Ok(None);
        }

        let Some(extension) = Path::new(&stored.relative_path)
            .extension()
            .and_then(|value| value.to_str())
        else {
            return Ok(None);
        };

        if !matches!(
            extension.to_ascii_lowercase().as_str(),
            "jpg" | "jpeg" | "png"
        ) {
            return Ok(None);
        }

        let Ok(image) = image::open(&stored.absolute_path) else {
            return Ok(None);
        };

        let media_directory = self.media_storage_directory();
        let filename =
            media_storage_filename_from_extension(&format!("{uuid}-thumb"), Some(extension));
        let destination_path = media_directory.join(&filename);
        let thumbnail = if image.width() > 430 {
            image.resize(430, u32::MAX, FilterType::Lanczos3)
        } else {
            image
        };

        thumbnail.save(&destination_path).map_err(|error| {
            DbError::Validation(format!("thumbnail generation failed: {error}"))
        })?;

        let size = fs::metadata(&destination_path)?.len();

        Ok(Some(GeneratedConversion {
            engine: "ImageResize",
            path: format!("media/{filename}"),
            size: i64::try_from(size)
                .map_err(|_| DbError::Validation("thumbnail is too large to track".to_string()))?,
        }))
    }

    fn create_video_thumb_conversion(
        &self,
        uuid: &str,
        stored: &StoredMediaFile,
    ) -> Result<Option<GeneratedConversion>, DbError> {
        if !stored.mime_type.starts_with("video/") {
            return Ok(None);
        }

        let ffmpeg = media_tool_path("FFMPEG_PATH", "ffmpeg");
        let ffprobe = media_tool_path("FFPROBE_PATH", "ffprobe");
        let seconds = ffprobe_duration_seconds(&ffprobe, &stored.absolute_path)
            .map(|duration| {
                if duration > 0.0 {
                    5.0_f64.min(duration.floor())
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0);
        let media_directory = self.media_storage_directory();
        let filename = media_storage_filename_from_extension(&format!("{uuid}-thumb"), Some("jpg"));
        let destination_path = media_directory.join(&filename);

        let mut generated =
            run_ffmpeg_frame(&ffmpeg, &stored.absolute_path, seconds, &destination_path)?;

        if !generated && seconds > 0.0 {
            generated = run_ffmpeg_frame(&ffmpeg, &stored.absolute_path, 0.0, &destination_path)?;
        }

        if !generated {
            return Ok(None);
        }

        let size = fs::metadata(&destination_path)?.len();

        Ok(Some(GeneratedConversion {
            engine: "VideoThumb",
            path: format!("media/{filename}"),
            size: i64::try_from(size)
                .map_err(|_| DbError::Validation("thumbnail is too large to track".to_string()))?,
        }))
    }

    pub fn system_health_counts(&self) -> Result<SystemHealthCounts, DbError> {
        let connection = self.connection()?;

        Ok(SystemHealthCounts {
            unauthorized_accounts: count_matching_rows(&connection, "accounts", "authorized = 0")?,
            failed_posts: count_matching_rows(
                &connection,
                "posts",
                "deleted_at IS NULL AND status = 3",
            )?,
            pending_jobs: count_matching_rows(&connection, "job_queue", "status = 'pending'")?,
            processing_jobs: count_matching_rows(
                &connection,
                "job_queue",
                "status = 'processing'",
            )?,
            failed_jobs: count_matching_rows(&connection, "job_queue", "status = 'failed'")?,
            rate_limits: count_rows(&connection, "rate_limits")?,
        })
    }

    pub fn dashboard_summary(&self, generated_at: &str) -> Result<DashboardSummary, DbError> {
        if generated_at.trim().is_empty() {
            return Err(DbError::Validation("generated_at is required".to_string()));
        }

        let connection = self.connection()?;
        let provider_count =
            connection.query_row("SELECT COUNT(DISTINCT provider) FROM accounts", [], |row| {
                row.get(0)
            })?;

        Ok(DashboardSummary {
            generated_at: generated_at.to_string(),
            accounts: DashboardAccountCounts {
                total: count_rows(&connection, "accounts")?,
                authorized: count_matching_rows(&connection, "accounts", "authorized = 1")?,
                unauthorized: count_matching_rows(&connection, "accounts", "authorized = 0")?,
                providers: provider_count,
            },
            posts: DashboardPostCounts {
                draft: count_matching_rows(
                    &connection,
                    "posts",
                    "deleted_at IS NULL AND status = 0",
                )?,
                scheduled: count_matching_rows(
                    &connection,
                    "posts",
                    "deleted_at IS NULL AND status = 1",
                )?,
                publishing: count_matching_rows(
                    &connection,
                    "posts",
                    "deleted_at IS NULL AND schedule_status = 1",
                )?,
                published: count_matching_rows(
                    &connection,
                    "posts",
                    "deleted_at IS NULL AND status = 2",
                )?,
                failed: count_matching_rows(
                    &connection,
                    "posts",
                    "deleted_at IS NULL AND status = 3",
                )?,
            },
            jobs: DashboardJobCounts {
                pending: count_matching_rows(&connection, "job_queue", "status = 'pending'")?,
                processing: count_matching_rows(&connection, "job_queue", "status = 'processing'")?,
                failed: count_matching_rows(&connection, "job_queue", "status = 'failed'")?,
            },
            providers: self.dashboard_provider_summaries(&connection)?,
            upcoming_posts: self.dashboard_posts(
                &connection,
                "p.deleted_at IS NULL AND p.status = 1",
                "ORDER BY datetime(p.scheduled_at) ASC, p.id ASC",
                5,
            )?,
            failed_posts: self.dashboard_posts(
                &connection,
                "p.deleted_at IS NULL AND p.status = 3",
                "ORDER BY datetime(p.updated_at) DESC, p.id DESC",
                5,
            )?,
        })
    }

    pub fn clear_resolved_system_state(
        &self,
        now: &str,
    ) -> Result<SystemMaintenanceSummary, DbError> {
        if now.trim().is_empty() {
            return Err(DbError::Validation("now is required".to_string()));
        }

        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let completed_jobs_deleted = transaction.execute(
            "DELETE FROM job_queue
             WHERE status = 'completed'",
            [],
        )?;
        let cancelled_jobs_deleted = transaction.execute(
            "DELETE FROM job_queue
             WHERE status = 'cancelled'",
            [],
        )?;
        let expired_rate_limits_cleared = transaction.execute(
            "DELETE FROM rate_limits
             WHERE retry_after_at <= ?1",
            params![now],
        )?;

        transaction.commit()?;

        Ok(SystemMaintenanceSummary {
            completed_jobs_deleted,
            cancelled_jobs_deleted,
            expired_rate_limits_cleared,
        })
    }

    pub fn recover_stale_processing_jobs(
        &self,
        now: &str,
        stale_before: &str,
    ) -> Result<StaleJobRecoverySummary, DbError> {
        if now.trim().is_empty() {
            return Err(DbError::Validation("now is required".to_string()));
        }

        if stale_before.trim().is_empty() {
            return Err(DbError::Validation("stale_before is required".to_string()));
        }

        let connection = self.connection()?;
        let requeued_jobs = connection.execute(
            "UPDATE job_queue
             SET status = 'pending',
                 run_at = ?1,
                 locked_at = NULL,
                 last_error = 'requeued after stale processing lock',
                 updated_at = CURRENT_TIMESTAMP
             WHERE status = 'processing'
               AND datetime(COALESCE(locked_at, updated_at)) <= datetime(?2)",
            params![now, stale_before],
        )?;

        Ok(StaleJobRecoverySummary { requeued_jobs })
    }

    pub fn account_report(&self, request: &ReportRequest) -> Result<ReportSnapshot, DbError> {
        let request = request.validated().map_err(DbError::Validation)?;

        self.account_report_for_end_date(
            request.account_id,
            &request.period,
            request.days,
            Utc::now().date_naive(),
        )
    }

    pub fn save_metric(&self, form: &MetricForm) -> Result<bool, DbError> {
        let connection = self.connection()?;
        let metric = form.validated().map_err(DbError::Validation)?;

        connection.execute(
            "INSERT INTO metrics (account_id, data_json, date)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id, date) DO UPDATE SET data_json = excluded.data_json",
            params![metric.account_id, metric.data_json, metric.date],
        )?;

        Ok(true)
    }

    pub fn save_audience(&self, form: &AudienceForm) -> Result<bool, DbError> {
        let connection = self.connection()?;
        let audience = form.validated().map_err(DbError::Validation)?;

        connection.execute(
            "INSERT INTO audience (account_id, total, date)
             VALUES (?1, ?2, ?3)",
            params![audience.account_id, audience.total, audience.date],
        )?;

        Ok(true)
    }

    pub fn save_facebook_insight(&self, form: &FacebookInsightForm) -> Result<bool, DbError> {
        let connection = self.connection()?;
        let insight = form.validated().map_err(DbError::Validation)?;

        connection.execute(
            "INSERT INTO facebook_insights (account_id, type, value, date, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(account_id, type, date) DO UPDATE SET
                value = excluded.value,
                updated_at = CURRENT_TIMESTAMP",
            params![
                insight.account_id,
                insight.insight_type,
                insight.value,
                insight.date
            ],
        )?;

        Ok(true)
    }

    pub fn settings(&self) -> Result<AppSettings, DbError> {
        let connection = self.connection()?;
        let mut settings = AppSettings::default();

        settings.timezone = self
            .setting_value(&connection, "timezone")?
            .unwrap_or(settings.timezone);
        settings.date_format = self
            .setting_value(&connection, "date_format")?
            .unwrap_or(settings.date_format);
        settings.time_format = self
            .setting_value(&connection, "time_format")?
            .unwrap_or(settings.time_format);
        settings.week_starts_on = self
            .setting_value(&connection, "week_starts_on")?
            .unwrap_or(settings.week_starts_on);
        settings.desktop_notifications = self
            .setting_value(&connection, "desktop_notifications")?
            .unwrap_or(settings.desktop_notifications);
        settings.operator_name = self
            .setting_value(&connection, "operator_name")?
            .unwrap_or(settings.operator_name);
        settings.admin_email = self
            .setting_value(&connection, "admin_email")?
            .unwrap_or(settings.admin_email);
        settings.default_accounts = self
            .setting_value(&connection, "default_accounts")?
            .unwrap_or(settings.default_accounts);

        Ok(settings)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), DbError> {
        let connection = self.connection()?;

        self.save_setting_value(&connection, "timezone", &settings.timezone)?;
        self.save_setting_value(&connection, "date_format", &settings.date_format)?;
        self.save_setting_value(&connection, "time_format", &settings.time_format)?;
        self.save_setting_value(&connection, "week_starts_on", &settings.week_starts_on)?;
        self.save_setting_value(
            &connection,
            "desktop_notifications",
            &settings.desktop_notifications,
        )?;
        self.save_setting_value(&connection, "operator_name", &settings.operator_name)?;
        self.save_setting_value(&connection, "admin_email", &settings.admin_email)?;
        self.save_setting_value(&connection, "default_accounts", &settings.default_accounts)?;

        Ok(())
    }

    pub fn local_data_snapshot(&self) -> Result<LocalDataSnapshot, DbError> {
        let connection = self.connection()?;

        Ok(LocalDataSnapshot {
            accounts: self.list_accounts(&connection, 20)?,
            services: self.list_services(&connection, 20)?,
            posts: self.list_posts(&connection, 20)?,
            media: self.list_media(&connection, 20)?,
            tags: self.list_tags(&connection, 20)?,
            jobs: self.list_jobs(&connection, 20)?,
            rate_limits: self.list_rate_limits(&connection, 20)?,
        })
    }

    pub fn record_system_log(
        &self,
        level: &str,
        message: &str,
        context: Option<serde_json::Value>,
    ) -> Result<(), DbError> {
        let level = normalize_system_log_level(level)?;
        let message = message.trim();

        if message.is_empty() {
            return Err(DbError::Validation("log message is required".to_string()));
        }

        let context_json = context
            .map(redact_diagnostics_value)
            .map(|value| serde_json::to_string(&value))
            .transpose()?;
        let connection = self.connection()?;

        connection.execute(
            "INSERT INTO system_logs (level, message, context_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![level, message, context_json, Utc::now().to_rfc3339()],
        )?;

        Ok(())
    }

    pub fn system_logs(&self) -> Result<Vec<SystemLogFile>, DbError> {
        let connection = self.connection()?;
        let (contents, entry_count) = self.system_log_contents(&connection)?;

        if entry_count == 0 {
            return Ok(Vec::new());
        }

        let total_bytes = contents.as_bytes().len();
        let preview = truncate_utf8_to_bytes(&contents, SYSTEM_LOG_PREVIEW_BYTES);
        let error = if total_bytes > SYSTEM_LOG_PREVIEW_BYTES {
            Some(format!(
                "Warning: system log file {SYSTEM_LOG_NAME} is {}; preview is limited to {}.",
                format_system_log_size(total_bytes),
                format_system_log_size(SYSTEM_LOG_PREVIEW_BYTES)
            ))
        } else {
            None
        };

        Ok(vec![SystemLogFile {
            name: SYSTEM_LOG_NAME.to_string(),
            contents: preview,
            error,
            bytes: u64::try_from(total_bytes).map_err(|_| {
                DbError::Validation("system log is too large to report".to_string())
            })?,
            entry_count,
        }])
    }

    pub fn export_system_log(&self) -> Result<SystemLogExportSummary, DbError> {
        self.record_system_log("info", "Exported system log", None)?;

        let generated_at = Utc::now();
        let connection = self.connection()?;
        let (contents, _) = self.system_log_contents(&connection)?;
        let directory = self
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("logs");
        let path = directory.join(format!(
            "dust-wave-system-{}.log",
            generated_at.format("%Y%m%dT%H%M%SZ")
        ));

        fs::create_dir_all(&directory)?;
        fs::write(&path, contents.as_bytes())?;

        Ok(SystemLogExportSummary {
            path: path.display().to_string(),
            bytes: u64::try_from(contents.len()).map_err(|_| {
                DbError::Validation("system log export is too large to report".to_string())
            })?,
        })
    }

    pub fn clear_system_logs(&self) -> Result<SystemLogClearSummary, DbError> {
        let connection = self.connection()?;
        let deleted_entries = connection.execute("DELETE FROM system_logs", [])?;

        Ok(SystemLogClearSummary { deleted_entries })
    }

    pub fn export_local_backup(&self) -> Result<LocalBackupExportSummary, DbError> {
        let generated_at = Utc::now();
        let app_data_directory = self
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let export_id = Uuid::new_v4().to_string();
        let backup_directory = app_data_directory.join("backups").join(format!(
            "dust-wave-backup-{}-{}",
            generated_at.format("%Y%m%dT%H%M%SZ"),
            &export_id[..8]
        ));
        let database_path = backup_directory.join("dust-wave-social.sqlite3");
        let media_path = backup_directory.join("media");
        let manifest_path = backup_directory.join("manifest.json");

        fs::create_dir_all(&backup_directory)?;
        fs::copy(&self.path, &database_path)?;
        let database_bytes = fs::metadata(&database_path)?.len();
        let (media_files, media_bytes) =
            copy_directory_contents(&self.media_storage_directory(), &media_path)?;
        let manifest = serde_json::json!({
            "generated_at": generated_at.to_rfc3339(),
            "product": "Dust Wave Social",
            "schema_version": CURRENT_SCHEMA_VERSION,
            "database": database_path.file_name().and_then(|value| value.to_str()).unwrap_or("dust-wave-social.sqlite3"),
            "media": "media",
            "media_files": media_files,
            "bytes": {
                "database": database_bytes,
                "media": media_bytes
            },
            "secrets": {
                "included": false,
                "detail": "OS keychain service credentials and account tokens are not included in local backups."
            }
        });
        let manifest_content = serde_json::to_vec_pretty(&manifest)?;

        fs::write(&manifest_path, &manifest_content)?;

        Ok(LocalBackupExportSummary {
            path: backup_directory.display().to_string(),
            database_path: database_path.display().to_string(),
            media_path: media_path.display().to_string(),
            manifest_path: manifest_path.display().to_string(),
            media_files,
            bytes: database_bytes
                + media_bytes
                + u64::try_from(manifest_content.len()).map_err(|_| {
                    DbError::Validation("backup manifest is too large to report".to_string())
                })?,
        })
    }

    pub fn restore_local_backup(
        &self,
        form: &LocalBackupRestoreForm,
    ) -> Result<LocalBackupRestoreSummary, DbError> {
        let request = form.validated().map_err(DbError::Validation)?;
        let backup_directory = PathBuf::from(&request.backup_path);

        if !backup_directory.is_dir() {
            return Err(DbError::Validation(
                "backup_path must point to a Dust Wave backup directory".to_string(),
            ));
        }

        let database_path = backup_directory.join("dust-wave-social.sqlite3");
        let media_path = backup_directory.join("media");
        let manifest_path = backup_directory.join("manifest.json");

        if !manifest_path.is_file() {
            return Err(DbError::Validation(
                "backup manifest.json is required".to_string(),
            ));
        }

        validate_backup_manifest(&manifest_path)?;

        if !database_path.is_file() {
            return Err(DbError::Validation(
                "backup database dust-wave-social.sqlite3 is required".to_string(),
            ));
        }

        if !media_path.is_dir() {
            return Err(DbError::Validation(
                "backup media folder is required".to_string(),
            ));
        }

        validate_backup_database(&database_path)?;

        let safety_backup = self.export_local_backup()?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(&database_path, &self.path)?;

        let current_media_directory = self.media_storage_directory();
        if current_media_directory.exists() {
            fs::remove_dir_all(&current_media_directory)?;
        }

        let (restored_media_files, restored_media_bytes) =
            copy_directory_contents(&media_path, &current_media_directory)?;

        self.migrate()?;
        let database_bytes = fs::metadata(&self.path)?.len();
        let manifest_bytes = fs::metadata(&manifest_path)?.len();
        let restored_bytes = database_bytes + restored_media_bytes + manifest_bytes;

        let _ = self.record_system_log(
            "warning",
            "Restored local backup",
            Some(serde_json::json!({
                "backup_path": backup_directory.display().to_string(),
                "safety_backup_path": safety_backup.path,
                "restored_media_files": restored_media_files,
            })),
        );

        Ok(LocalBackupRestoreSummary {
            backup_path: backup_directory.display().to_string(),
            safety_backup_path: safety_backup.path,
            database_path: self.path.display().to_string(),
            media_path: current_media_directory.display().to_string(),
            restored_media_files,
            restored_bytes,
        })
    }

    fn system_log_contents(&self, connection: &Connection) -> Result<(String, usize), DbError> {
        let mut statement = connection.prepare(
            "SELECT level, message, context_json, created_at
             FROM system_logs
             ORDER BY datetime(created_at) ASC, id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut contents = String::new();
        let mut entry_count = 0;

        for row in rows {
            let (level, message, context_json, created_at) = row?;
            entry_count += 1;
            contents.push_str(&created_at);
            contents.push_str(" [");
            contents.push_str(&level.to_ascii_uppercase());
            contents.push_str("] ");
            contents.push_str(&message.replace('\n', "\\n"));

            if let Some(context_json) = context_json {
                contents.push(' ');
                contents.push_str(&context_json);
            }

            contents.push('\n');
        }

        Ok((contents, entry_count))
    }

    pub fn query_posts(&self, request: &PostQueryRequest) -> Result<PostQueryResult, DbError> {
        let mut request = request.validated().map_err(DbError::Validation)?;
        let connection = self.connection()?;
        let calendar_window = self.calendar_window(&connection, &request)?;
        let (where_sql, values) = self.post_query_filter(&request, calendar_window.as_ref())?;
        let total = self.count_matching_posts(&connection, &where_sql, &values)?;
        let total_pages = if total == 0 {
            1
        } else {
            ((total + request.limit - 1) / request.limit).max(1)
        };
        request.page = request.page.min(total_pages);
        let items = self.filtered_posts(&connection, &where_sql, &values, &request)?;

        Ok(PostQueryResult {
            items,
            total,
            page: request.page,
            per_page: request.limit,
            total_pages,
            has_failed_posts: self.has_failed_posts(&connection)?,
            calendar_window,
        })
    }

    #[cfg(test)]
    pub fn accounts(&self) -> Result<Vec<AccountSummary>, DbError> {
        let connection = self.connection()?;

        self.list_accounts(&connection, 100)
    }

    pub fn save_account(&self, form: &AccountForm) -> Result<AccountSummary, DbError> {
        let connection = self.connection()?;
        let account = form.validated().map_err(DbError::Validation)?;
        let provider = account.provider.clone();
        let provider_id = account.provider_id.clone();

        connection.execute(
            "INSERT INTO accounts (
                uuid, name, username, avatar_path, provider, provider_id, data_json, authorized,
                access_token_secret_ref, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
             )
             ON CONFLICT(provider, provider_id) DO UPDATE SET
                name = excluded.name,
                username = excluded.username,
                avatar_path = excluded.avatar_path,
                data_json = COALESCE(excluded.data_json, accounts.data_json),
                authorized = excluded.authorized,
                access_token_secret_ref = excluded.access_token_secret_ref,
                updated_at = CURRENT_TIMESTAMP",
            params![
                Uuid::new_v4().to_string(),
                account.name,
                account.username,
                account.avatar_path,
                account.provider,
                account.provider_id,
                account.data_json,
                if account.authorized { 1 } else { 0 },
                account.access_token_secret_ref
            ],
        )?;

        self.account_by_provider_id(&connection, &provider, &provider_id)
    }

    pub fn delete_account(&self, uuid: &str) -> Result<bool, DbError> {
        let connection = self.connection()?;
        let changed = connection.execute("DELETE FROM accounts WHERE uuid = ?1", params![uuid])?;

        Ok(changed > 0)
    }

    pub fn refresh_mastodon_account(&self, uuid: &str) -> Result<AccountSummary, DbError> {
        let connection = self.connection()?;
        let account: (String, String, Option<String>, String) = connection.query_row(
            "SELECT provider, provider_id, data_json, access_token_secret_ref
             FROM accounts
             WHERE uuid = ?1",
            params![uuid],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        drop(connection);

        let (provider, provider_id, data_json, access_token_secret_ref) = account;

        if provider != "mastodon" {
            return Err(DbError::Validation(
                "only Mastodon accounts can be refreshed by this command".to_string(),
            ));
        }

        let server = mastodon_server_from_account_data(data_json.as_deref())?.ok_or_else(|| {
            DbError::Validation("Mastodon account must be connected before refresh".to_string())
        })?;
        let access_token = match resolve_account_secret("mastodon", &provider_id, "access_token") {
            Ok(value) => value,
            Err(error) => {
                self.set_account_authorized(uuid, false)?;
                return Err(DbError::Validation(error.to_string()));
            }
        };
        let form = match verify_mastodon_account(&server, &access_token, access_token_secret_ref) {
            Ok(form) => form,
            Err(error) => {
                if should_mark_mastodon_unauthorized(&error) {
                    self.set_account_authorized(uuid, false)?;
                }
                return Err(DbError::Validation(error.to_string()));
            }
        };

        self.save_account(&form)
    }

    pub fn refresh_twitter_account(&self, uuid: &str) -> Result<AccountSummary, DbError> {
        let connection = self.connection()?;
        let account: (String, String, Option<String>, String) = connection.query_row(
            "SELECT provider, provider_id, data_json, access_token_secret_ref
             FROM accounts
             WHERE uuid = ?1",
            params![uuid],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        drop(connection);

        let (provider, provider_id, data_json, access_token_secret_ref) = account;

        if provider != "twitter" {
            return Err(DbError::Validation(
                "only X/Twitter accounts can be refreshed by this command".to_string(),
            ));
        }

        if !twitter_oauth2_account_data(data_json.as_deref())? {
            return Err(DbError::Validation(
                "X/Twitter account must be connected with OAuth 2.0 before refresh".to_string(),
            ));
        }

        let access_token = match resolve_account_secret("twitter", &provider_id, "access_token") {
            Ok(value) => value,
            Err(error) => {
                self.set_account_authorized(uuid, false)?;
                return Err(DbError::Validation(error.to_string()));
            }
        };
        let form = match verify_twitter_account(
            &access_token,
            access_token_secret_ref,
            data_json.as_deref(),
        ) {
            Ok(form) => form,
            Err(error) => {
                if should_mark_twitter_unauthorized(&error) {
                    self.set_account_authorized(uuid, false)?;
                }
                return Err(DbError::Validation(error.to_string()));
            }
        };

        self.save_account(&form)
    }

    pub fn import_twitter_account_data(&self, uuid: &str) -> Result<TwitterImportSummary, DbError> {
        let connection = self.connection()?;
        let account: (i64, String, String, String, Option<String>) = connection.query_row(
            "SELECT id, provider, provider_id, access_token_secret_ref, data_json
             FROM accounts
             WHERE uuid = ?1",
            params![uuid],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;

        drop(connection);

        let (account_id, provider, provider_id, _access_token_secret_ref, data_json) = account;

        if provider != "twitter" {
            return Err(DbError::Validation(
                "only X/Twitter accounts can be imported by this command".to_string(),
            ));
        }

        if !twitter_oauth2_account_data(data_json.as_deref())? {
            return Err(DbError::Validation(
                "X/Twitter account must be connected with OAuth 2.0 before import".to_string(),
            ));
        }

        let access_token = match resolve_account_secret("twitter", &provider_id, "access_token") {
            Ok(value) => value,
            Err(error) => {
                self.set_account_authorized(uuid, false)?;
                return Err(DbError::Validation(error.to_string()));
            }
        };
        let account_metrics = match fetch_twitter_account_metrics(&access_token) {
            Ok(value) => value,
            Err(error) => {
                if should_mark_twitter_unauthorized(&error) {
                    self.set_account_authorized(uuid, false)?;
                }
                return Err(DbError::Validation(error.to_string()));
            }
        };
        let twitter_tier = self
            .service_configuration_value("twitter", "tier")?
            .unwrap_or_else(|| "legacy".to_string());
        let posts = if twitter_tier == "free" {
            Vec::new()
        } else {
            match fetch_twitter_user_posts(&access_token, &provider_id) {
                Ok(value) => value,
                Err(error) => {
                    if should_mark_twitter_unauthorized(&error) {
                        self.set_account_authorized(uuid, false)?;
                    }
                    return Err(DbError::Validation(error.to_string()));
                }
            }
        };
        let today = Utc::now().date_naive().to_string();
        let audience_total = account_metrics
            .get("followers_count")
            .and_then(json_number_to_i64);
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;

        if let Some(total) = audience_total {
            transaction.execute(
                "DELETE FROM audience
                 WHERE account_id = ?1 AND date = ?2",
                params![account_id, today],
            )?;
            transaction.execute(
                "INSERT INTO audience (account_id, total, date)
                 VALUES (?1, ?2, ?3)",
                params![account_id, total, today],
            )?;
        }

        let imported_posts = import_twitter_post_rows(&transaction, account_id, &posts)?;
        let metric_days = process_twitter_metric_days(&transaction, account_id)?;
        transaction.commit()?;
        let account = self.account_by_provider_id(&self.connection()?, "twitter", &provider_id)?;

        Ok(TwitterImportSummary {
            account,
            audience_total,
            imported_posts,
            metric_days,
        })
    }

    pub fn refresh_facebook_page_account(&self, uuid: &str) -> Result<AccountSummary, DbError> {
        let connection = self.connection()?;
        let account: (String, String, Option<String>, String) = connection.query_row(
            "SELECT provider, provider_id, data_json, access_token_secret_ref
             FROM accounts
             WHERE uuid = ?1",
            params![uuid],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        drop(connection);

        let (provider, provider_id, data_json, access_token_secret_ref) = account;

        if provider != "facebook_page" {
            return Err(DbError::Validation(
                "only Facebook Page accounts can be refreshed by this command".to_string(),
            ));
        }

        if !facebook_page_connected_account_data(data_json.as_deref())? {
            return Err(DbError::Validation(
                "Facebook Page account must be connected before refresh".to_string(),
            ));
        }

        let page_access_token =
            match resolve_account_secret("facebook_page", &provider_id, "page_access_token") {
                Ok(value) => value,
                Err(error) => {
                    self.set_account_authorized(uuid, false)?;
                    return Err(DbError::Validation(error.to_string()));
                }
            };
        let api_version = self.service_configuration_value("facebook", "api_version")?;
        let form = match verify_facebook_page_account(
            &provider_id,
            &page_access_token,
            access_token_secret_ref,
            data_json.as_deref(),
            api_version.as_deref(),
        ) {
            Ok(form) => form,
            Err(error) => {
                if should_mark_facebook_unauthorized(&error) {
                    self.set_account_authorized(uuid, false)?;
                }
                return Err(DbError::Validation(error.to_string()));
            }
        };

        self.save_account(&form)
    }

    pub fn import_facebook_page_data(&self, uuid: &str) -> Result<FacebookImportSummary, DbError> {
        let connection = self.connection()?;
        let account: (i64, String, String, Option<String>) = connection.query_row(
            "SELECT id, provider, provider_id, data_json
             FROM accounts
             WHERE uuid = ?1",
            params![uuid],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        drop(connection);

        let (account_id, provider, provider_id, data_json) = account;

        if provider != "facebook_page" {
            return Err(DbError::Validation(
                "only Facebook Page accounts can be imported by this command".to_string(),
            ));
        }

        if !facebook_page_connected_account_data(data_json.as_deref())? {
            return Err(DbError::Validation(
                "Facebook Page account must be connected before import".to_string(),
            ));
        }

        let page_access_token =
            match resolve_account_secret("facebook_page", &provider_id, "page_access_token") {
                Ok(value) => value,
                Err(error) => {
                    self.set_account_authorized(uuid, false)?;
                    return Err(DbError::Validation(error.to_string()));
                }
            };
        let api_version = self.service_configuration_value("facebook", "api_version")?;
        let audience = match fetch_facebook_page_audience(
            &provider_id,
            &page_access_token,
            api_version.as_deref(),
        ) {
            Ok(value) => value,
            Err(error) => {
                if should_mark_facebook_unauthorized(&error) {
                    self.set_account_authorized(uuid, false)?;
                }
                return Err(DbError::Validation(error.to_string()));
            }
        };
        let insights = match fetch_facebook_page_insights(
            &provider_id,
            &page_access_token,
            api_version.as_deref(),
        ) {
            Ok(value) => value,
            Err(error) => {
                if should_mark_facebook_unauthorized(&error) {
                    self.set_account_authorized(uuid, false)?;
                }
                return Err(DbError::Validation(error.to_string()));
            }
        };
        let today = Utc::now().date_naive().to_string();
        let audience_total = audience
            .get("followers_count")
            .or_else(|| audience.get("fan_count"))
            .and_then(json_number_to_i64);
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;

        if let Some(total) = audience_total {
            transaction.execute(
                "DELETE FROM audience
                 WHERE account_id = ?1 AND date = ?2",
                params![account_id, today],
            )?;
            transaction.execute(
                "INSERT INTO audience (account_id, total, date)
                 VALUES (?1, ?2, ?3)",
                params![account_id, total, today],
            )?;
        }

        let insight_rows = import_facebook_insight_rows(&transaction, account_id, &insights)?;
        transaction.commit()?;
        let account =
            self.account_by_provider_id(&self.connection()?, "facebook_page", &provider_id)?;

        Ok(FacebookImportSummary {
            account,
            audience_total,
            insight_rows,
        })
    }

    pub fn import_mastodon_account_data(
        &self,
        uuid: &str,
    ) -> Result<MastodonImportSummary, DbError> {
        let connection = self.connection()?;
        let account: (i64, String, String, String, Option<String>) = connection.query_row(
            "SELECT id, provider, provider_id, access_token_secret_ref, data_json
             FROM accounts
             WHERE uuid = ?1",
            params![uuid],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;

        drop(connection);

        let (account_id, provider, provider_id, _access_token_secret_ref, data_json) = account;

        if provider != "mastodon" {
            return Err(DbError::Validation(
                "only Mastodon accounts can be imported by this command".to_string(),
            ));
        }

        let server = mastodon_server_from_account_data(data_json.as_deref())?.ok_or_else(|| {
            DbError::Validation("Mastodon account must be connected before import".to_string())
        })?;
        let access_token = match resolve_account_secret("mastodon", &provider_id, "access_token") {
            Ok(value) => value,
            Err(error) => {
                self.set_account_authorized(uuid, false)?;
                return Err(DbError::Validation(error.to_string()));
            }
        };
        let account_metrics = match fetch_mastodon_account_metrics(&server, &access_token) {
            Ok(value) => value,
            Err(error) => {
                if should_mark_mastodon_unauthorized(&error) {
                    self.set_account_authorized(uuid, false)?;
                }
                return Err(DbError::Validation(error.to_string()));
            }
        };
        let statuses = match fetch_mastodon_user_statuses(&server, &access_token, &provider_id) {
            Ok(value) => value,
            Err(error) => {
                if should_mark_mastodon_unauthorized(&error) {
                    self.set_account_authorized(uuid, false)?;
                }
                return Err(DbError::Validation(error.to_string()));
            }
        };
        let today = Utc::now().date_naive().to_string();
        let audience_total = json_number_to_i64(
            account_metrics
                .get("followers_count")
                .unwrap_or(&serde_json::Value::Null),
        );
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;

        if let Some(total) = audience_total {
            transaction.execute(
                "DELETE FROM audience
                 WHERE account_id = ?1 AND date = ?2",
                params![account_id, today],
            )?;
            transaction.execute(
                "INSERT INTO audience (account_id, total, date)
                 VALUES (?1, ?2, ?3)",
                params![account_id, total, today],
            )?;
        }

        let imported_posts = import_mastodon_status_rows(&transaction, account_id, &statuses)?;
        let metric_days = process_mastodon_metric_days(&transaction, account_id)?;
        transaction.commit()?;
        let account = self.account_by_provider_id(&self.connection()?, "mastodon", &provider_id)?;

        Ok(MastodonImportSummary {
            account,
            audience_total,
            imported_posts,
            metric_days,
        })
    }

    pub fn enqueue_account_import(&self, uuid: &str, run_at: &str) -> Result<JobSummary, DbError> {
        if run_at.trim().is_empty() {
            return Err(DbError::Validation("run_at is required".to_string()));
        }

        let connection = self.connection()?;
        let (account_id, provider): (i64, String) = connection.query_row(
            "SELECT id, provider
             FROM accounts
             WHERE uuid = ?1",
            params![uuid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        drop(connection);

        if supports_account_imports(&provider) {
            self.enqueue_job(&JobForm {
                kind: "import_account_data".to_string(),
                payload: serde_json::json!({
                    "account_id": account_id,
                    "provider": provider,
                }),
                run_at: run_at.to_string(),
                idempotency_key: Some(format!("import_account:{account_id}")),
            })
        } else {
            Err(DbError::Validation(format!(
                "{provider} imports are not supported yet"
            )))
        }
    }

    pub fn enqueue_all_account_imports(
        &self,
        run_at: &str,
    ) -> Result<AccountImportQueueBatchSummary, DbError> {
        if run_at.trim().is_empty() {
            return Err(DbError::Validation("run_at is required".to_string()));
        }

        let accounts = {
            let connection = self.connection()?;
            let mut statement = connection.prepare(
                "SELECT id, provider, authorized
                 FROM accounts
                 ORDER BY updated_at DESC, id DESC",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? == 1,
                ))
            })?;

            collect_rows(rows)?
        };

        let mut summary = AccountImportQueueBatchSummary {
            requested_accounts: accounts.len(),
            ..AccountImportQueueBatchSummary::default()
        };

        for (account_id, provider, authorized) in accounts {
            if !supports_account_imports(&provider) {
                summary.skipped_unsupported += 1;
                continue;
            }

            if !authorized {
                summary.skipped_unauthorized += 1;
                continue;
            }

            summary.eligible_accounts += 1;
            let job = self.enqueue_job(&JobForm {
                kind: "import_account_data".to_string(),
                payload: serde_json::json!({
                    "account_id": account_id,
                    "provider": provider,
                }),
                run_at: run_at.to_string(),
                idempotency_key: Some(format!("import_account:{account_id}")),
            })?;
            summary.jobs.push(job);
        }

        summary.queued_jobs = summary.jobs.len();

        Ok(summary)
    }

    #[cfg(test)]
    pub fn media(&self) -> Result<Vec<MediaSummary>, DbError> {
        let connection = self.connection()?;

        self.list_media(&connection, 100)
    }

    #[cfg(test)]
    pub fn media_library(&self) -> Result<Vec<MediaLibraryItem>, DbError> {
        self.query_media_library(&MediaLibraryRequest::default())
    }

    pub fn query_media_library(
        &self,
        request: &MediaLibraryRequest,
    ) -> Result<Vec<MediaLibraryItem>, DbError> {
        let connection = self.connection()?;
        let request = request.validated().map_err(DbError::Validation)?;

        self.list_media_library(&connection, &request)
    }

    #[cfg(test)]
    pub fn create_media(&self, form: &MediaForm) -> Result<MediaSummary, DbError> {
        let connection = self.connection()?;
        let media = form.validated().map_err(DbError::Validation)?;
        let uuid = Uuid::new_v4().to_string();

        connection.execute(
            "INSERT INTO media (
                uuid, name, mime_type, disk, path, data_json, size, size_total,
                conversions_json, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
             )",
            params![
                &uuid,
                media.name,
                media.mime_type,
                media.disk,
                media.path,
                media.data_json,
                media.size,
                media.size_total,
                media.conversions_json
            ],
        )?;

        self.media_by_uuid(&connection, &uuid)
    }

    pub fn import_media_file(&self, form: &MediaImportForm) -> Result<MediaSummary, DbError> {
        let media = form.validated().map_err(DbError::Validation)?;
        let source_path = Path::new(&media.source_path);
        let metadata = fs::metadata(source_path)?;

        if !metadata.is_file() {
            return Err(DbError::Validation(
                "source_path must point to a file".to_string(),
            ));
        }

        let original_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("imported-media")
            .to_string();
        let name = media.name.unwrap_or_else(|| original_name.clone());
        let uuid = Uuid::new_v4().to_string();
        let filename = media_storage_filename(&uuid, source_path);
        let media_directory = self.media_storage_directory();
        let destination_path = media_directory.join(&filename);
        let relative_path = format!("media/{filename}");
        let size = i64::try_from(metadata.len()).map_err(|_| {
            DbError::Validation("source file is too large to track as media".to_string())
        })?;
        let mime_type = infer_media_mime_type(source_path).to_string();
        validate_media_upload(&mime_type, metadata.len())?;
        let data_json = serde_json::to_string(&serde_json::json!({
            "source": "local-file-import",
            "original_name": original_name,
        }))?;
        let connection = self.connection()?;

        fs::create_dir_all(&media_directory)?;
        fs::copy(source_path, &destination_path)?;
        let stored = StoredMediaFile {
            original_name,
            mime_type,
            relative_path,
            absolute_path: destination_path,
            size,
        };
        let conversion_artifacts = self.media_conversion_artifacts(&uuid, &stored)?;

        connection.execute(
            "INSERT INTO media (
                uuid, name, mime_type, disk, path, data_json, size, size_total,
                conversions_json, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, 'local', ?4, ?5, ?6, ?7, ?8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
             )",
            params![
                &uuid,
                name,
                stored.mime_type,
                stored.relative_path,
                data_json,
                stored.size,
                conversion_artifacts.size_total,
                conversion_artifacts.conversions_json
            ],
        )?;

        self.media_by_uuid(&connection, &uuid)
    }

    pub fn download_external_media(
        &self,
        form: &MediaDownloadForm,
    ) -> Result<MediaSummary, DbError> {
        let media = form.validated().map_err(DbError::Validation)?;
        let url = Url::parse(&media.url)
            .map_err(|error| DbError::Validation(format!("invalid url: {error}")))?;
        let is_klipy_download = media
            .download_data
            .as_ref()
            .and_then(|value| value.get("provider"))
            .and_then(|value| value.as_str())
            .map(|provider| provider.eq_ignore_ascii_case("klipy"))
            .unwrap_or(false);

        if is_klipy_download {
            return Err(DbError::Validation(
                "Klipy GIFs cannot be saved to the reusable media library; use a Klipy provider reference and transient publish-time upload instead"
                    .to_string(),
            ));
        }

        let uuid = Uuid::new_v4().to_string();
        let stored = match url.scheme() {
            "file" => self.store_downloaded_file_url(&uuid, &url)?,
            "http" | "https" => self.store_downloaded_http_url(&uuid, &url)?,
            _ => {
                return Err(DbError::Validation(
                    "url must start with file://, http://, or https://".to_string(),
                ));
            }
        };
        let name = media.name.unwrap_or_else(|| stored.original_name.clone());
        let mut data = serde_json::Map::new();
        data.insert(
            "source".to_string(),
            serde_json::Value::String(media.source.clone()),
        );
        data.insert(
            "external_url".to_string(),
            serde_json::Value::String(media.url.clone()),
        );
        data.insert(
            "original_name".to_string(),
            serde_json::Value::String(stored.original_name.clone()),
        );

        if let Some(download_data) = media.download_data {
            data.insert("download_data".to_string(), download_data);
        }

        let data_json = serde_json::to_string(&serde_json::Value::Object(data))?;
        let conversion_artifacts = self.media_conversion_artifacts(&uuid, &stored)?;
        let connection = self.connection()?;

        connection.execute(
            "INSERT INTO media (
                uuid, name, mime_type, disk, path, data_json, size, size_total,
                conversions_json, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, 'local', ?4, ?5, ?6, ?7, ?8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
             )",
            params![
                &uuid,
                name,
                stored.mime_type,
                stored.relative_path,
                data_json,
                stored.size,
                conversion_artifacts.size_total,
                conversion_artifacts.conversions_json
            ],
        )?;

        self.media_by_uuid(&connection, &uuid)
    }

    pub fn delete_media(&self, uuid: &str) -> Result<bool, DbError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let Some(paths) = self.managed_media_file_paths_by_uuid(&transaction, uuid)? else {
            return Ok(false);
        };

        for path in paths {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(DbError::Io(error)),
            }
        }

        let changed = transaction.execute("DELETE FROM media WHERE uuid = ?1", params![uuid])?;
        transaction.commit()?;

        Ok(changed > 0)
    }

    pub fn cleanup_orphaned_media_files(&self) -> Result<MediaCleanupSummary, DbError> {
        let media_directory = self.media_storage_directory();

        if !media_directory.exists() {
            return Ok(MediaCleanupSummary::default());
        }

        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT disk, path, conversions_json FROM media")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        let mut referenced_paths = BTreeSet::new();

        for row in rows {
            let (disk, path, conversions_json) = row?;

            for path in self.managed_media_file_paths(&disk, &path, conversions_json.as_deref()) {
                referenced_paths.insert(path);
            }
        }

        let mut summary = MediaCleanupSummary::default();

        for entry in fs::read_dir(media_directory)? {
            let entry = entry?;

            if !entry.file_type()?.is_file() {
                continue;
            }

            summary.scanned += 1;
            let path = entry.path();

            if referenced_paths.contains(&path) {
                summary.retained += 1;
                continue;
            }

            let size = entry.metadata()?.len();
            fs::remove_file(&path)?;
            summary.deleted += 1;
            summary.reclaimed_bytes += i64::try_from(size).map_err(|_| {
                DbError::Validation("media file is too large to summarize".to_string())
            })?;
        }

        Ok(summary)
    }

    pub fn create_draft_post(&self, form: &PostForm) -> Result<PostSummary, DbError> {
        let mut connection = self.connection()?;
        let post = form.validated().map_err(DbError::Validation)?;
        let uuid = Uuid::new_v4().to_string();
        let transaction = connection.transaction()?;

        transaction.execute(
            "INSERT INTO posts (
                uuid, status, schedule_status, scheduled_at, created_at, updated_at
             ) VALUES (
                ?1, 0, 0, ?2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
             )",
            params![uuid, post.scheduled_at],
        )?;

        let post_id = transaction.last_insert_rowid();

        for account_id in post.accounts {
            transaction.execute(
                "INSERT INTO post_accounts (post_id, account_id) VALUES (?1, ?2)",
                params![post_id, account_id],
            )?;
        }

        for tag_id in post.tags {
            transaction.execute(
                "INSERT INTO tag_post (tag_id, post_id) VALUES (?1, ?2)",
                params![tag_id, post_id],
            )?;
        }

        for version in post.versions {
            transaction.execute(
                "INSERT INTO post_versions (post_id, account_id, is_original, content_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    post_id,
                    version.account_id,
                    if version.is_original { 1 } else { 0 },
                    version.content_json
                ],
            )?;
        }

        transaction.commit()?;

        self.post_by_id(&connection, post_id)
    }

    pub fn post_detail(&self, uuid: &str) -> Result<PostDetail, DbError> {
        let connection = self.connection()?;

        self.post_detail_by_uuid(&connection, uuid)
    }

    pub fn validate_post(&self, uuid: &str) -> Result<PostValidationReport, DbError> {
        let connection = self.connection()?;
        let post_id: i64 = connection
            .query_row(
                "SELECT id
                 FROM posts
                 WHERE uuid = ?1 AND deleted_at IS NULL",
                params![uuid],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| DbError::Validation("post not found".to_string()))?;

        self.validate_post_for_publish(&connection, post_id)
    }

    pub fn update_post(&self, uuid: &str, form: &PostForm) -> Result<PostSummary, DbError> {
        let mut connection = self.connection()?;
        let post = form.validated().map_err(DbError::Validation)?;
        let scheduled_at = post.scheduled_at.clone();
        let should_return_to_draft = post.accounts.is_empty() || scheduled_at.is_none();
        let transaction = connection.transaction()?;
        let (post_id, status, schedule_status) = transaction
            .query_row(
                "SELECT id, status, schedule_status
                 FROM posts
                 WHERE uuid = ?1 AND deleted_at IS NULL",
                params![uuid],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| DbError::Validation("post not found".to_string()))?;

        if matches!(status, 2 | 3) {
            return Err(DbError::Validation(
                "published or failed posts cannot be updated".to_string(),
            ));
        }

        if schedule_status == 1 {
            return Err(DbError::Validation(
                "posts currently publishing cannot be updated".to_string(),
            ));
        }

        transaction.execute(
            "DELETE FROM post_accounts WHERE post_id = ?1",
            params![post_id],
        )?;
        for account_id in &post.accounts {
            transaction.execute(
                "INSERT INTO post_accounts (post_id, account_id) VALUES (?1, ?2)",
                params![post_id, account_id],
            )?;
        }

        transaction.execute("DELETE FROM tag_post WHERE post_id = ?1", params![post_id])?;
        for tag_id in &post.tags {
            transaction.execute(
                "INSERT INTO tag_post (tag_id, post_id) VALUES (?1, ?2)",
                params![tag_id, post_id],
            )?;
        }

        transaction.execute(
            "DELETE FROM post_versions WHERE post_id = ?1",
            params![post_id],
        )?;
        for version in &post.versions {
            transaction.execute(
                "INSERT INTO post_versions (post_id, account_id, is_original, content_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    post_id,
                    version.account_id,
                    if version.is_original { 1 } else { 0 },
                    version.content_json
                ],
            )?;
        }

        if should_return_to_draft {
            transaction.execute(
                "UPDATE posts
                 SET status = 0,
                     schedule_status = 0,
                     scheduled_at = ?2,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![post_id, scheduled_at],
            )?;
            Self::cancel_pending_publish_jobs(&transaction, post_id, "post returned to draft")?;
        } else {
            transaction.execute(
                "UPDATE posts
                 SET scheduled_at = ?2,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![post_id, scheduled_at],
            )?;

            if status == 1 {
                Self::upsert_pending_publish_job(
                    &transaction,
                    post_id,
                    scheduled_at
                        .as_deref()
                        .expect("scheduled posts have a schedule time after validation"),
                )?;
            }
        }

        transaction.commit()?;

        self.post_by_id(&connection, post_id)
    }

    pub fn duplicate_post(&self, uuid: &str) -> Result<PostSummary, DbError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let source_id: i64 = transaction
            .query_row(
                "SELECT id
                 FROM posts
                 WHERE uuid = ?1 AND deleted_at IS NULL",
                params![uuid],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| DbError::Validation("post not found".to_string()))?;
        let new_uuid = Uuid::new_v4().to_string();

        transaction.execute(
            "INSERT INTO posts (
                uuid, status, schedule_status, created_at, updated_at
             ) VALUES (
                ?1, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
             )",
            params![new_uuid],
        )?;

        let post_id = transaction.last_insert_rowid();

        transaction.execute(
            "INSERT INTO post_accounts (post_id, account_id)
             SELECT ?1, account_id
             FROM post_accounts
             WHERE post_id = ?2
             ORDER BY id ASC",
            params![post_id, source_id],
        )?;
        transaction.execute(
            "INSERT INTO tag_post (tag_id, post_id)
             SELECT tag_id, ?1
             FROM tag_post
             WHERE post_id = ?2
             ORDER BY id ASC",
            params![post_id, source_id],
        )?;
        transaction.execute(
            "INSERT INTO post_versions (post_id, account_id, is_original, content_json)
             SELECT ?1, account_id, is_original, content_json
             FROM post_versions
             WHERE post_id = ?2
             ORDER BY id ASC",
            params![post_id, source_id],
        )?;

        transaction.commit()?;

        self.post_by_id(&connection, post_id)
    }

    pub fn schedule_post(
        &self,
        uuid: &str,
        form: &SchedulePostForm,
    ) -> Result<PostSummary, DbError> {
        let mut connection = self.connection()?;
        let scheduled_at = form.validated().map_err(DbError::Validation)?;
        let transaction = connection.transaction()?;
        let (post_id, status, schedule_status) = transaction
            .query_row(
                "SELECT id, status, schedule_status
                 FROM posts
                 WHERE uuid = ?1 AND deleted_at IS NULL",
                params![uuid],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| DbError::Validation("post not found".to_string()))?;

        if matches!(status, 2 | 3) {
            return Err(DbError::Validation(
                "published or failed posts cannot be scheduled".to_string(),
            ));
        }

        if schedule_status == 1 {
            return Err(DbError::Validation(
                "posts currently publishing cannot be scheduled".to_string(),
            ));
        }

        let account_count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM post_accounts WHERE post_id = ?1",
            params![post_id],
            |row| row.get(0),
        )?;

        if account_count == 0 {
            return Err(DbError::Validation(
                "post must have at least one account before scheduling".to_string(),
            ));
        }

        let validation = self.validate_post_for_publish(&transaction, post_id)?;
        if !validation.valid {
            return Err(DbError::Validation(format_validation_errors(&validation)));
        }

        transaction.execute(
            "UPDATE posts
             SET status = 1,
                 schedule_status = 0,
                 scheduled_at = ?2,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![post_id, &scheduled_at],
        )?;

        Self::upsert_pending_publish_job(&transaction, post_id, &scheduled_at)?;

        transaction.commit()?;

        self.post_by_id(&connection, post_id)
    }

    pub fn retry_failed_post(
        &self,
        uuid: &str,
        form: &SchedulePostForm,
    ) -> Result<PostSummary, DbError> {
        let mut connection = self.connection()?;
        let scheduled_at = form.validated().map_err(DbError::Validation)?;
        let transaction = connection.transaction()?;
        let (post_id, status, schedule_status) = transaction
            .query_row(
                "SELECT id, status, schedule_status
                 FROM posts
                 WHERE uuid = ?1 AND deleted_at IS NULL",
                params![uuid],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| DbError::Validation("post not found".to_string()))?;

        if status != 3 {
            return Err(DbError::Validation(
                "only failed posts can be retried".to_string(),
            ));
        }

        if schedule_status == 1 {
            return Err(DbError::Validation(
                "posts currently publishing cannot be retried".to_string(),
            ));
        }

        let account_count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM post_accounts WHERE post_id = ?1",
            params![post_id],
            |row| row.get(0),
        )?;

        if account_count == 0 {
            return Err(DbError::Validation(
                "post must have at least one account before retrying".to_string(),
            ));
        }

        let validation = self.validate_post_for_publish(&transaction, post_id)?;
        if !validation.valid {
            return Err(DbError::Validation(format_validation_errors(&validation)));
        }

        transaction.execute(
            "UPDATE post_accounts
             SET errors_json = NULL,
                 data_json = NULL,
                 provider_post_id = NULL
             WHERE post_id = ?1",
            params![post_id],
        )?;
        transaction.execute(
            "UPDATE posts
             SET status = 1,
                 schedule_status = 0,
                 scheduled_at = ?2,
                 published_at = NULL,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![post_id, &scheduled_at],
        )?;

        Self::upsert_pending_publish_job(&transaction, post_id, &scheduled_at)?;

        transaction.commit()?;

        self.post_by_id(&connection, post_id)
    }

    pub fn delete_post(&self, uuid: &str) -> Result<bool, DbError> {
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE posts
             SET deleted_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
             WHERE uuid = ?1 AND deleted_at IS NULL",
            params![uuid],
        )?;

        Ok(changed > 0)
    }

    pub fn bulk_delete_posts(
        &self,
        form: &BulkDeletePostsForm,
    ) -> Result<BulkDeletePostsSummary, DbError> {
        let uuids = form
            .uuids
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<BTreeSet<_>>();

        if uuids.is_empty() {
            return Err(DbError::Validation(
                "at least one post uuid is required".to_string(),
            ));
        }

        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let mut deleted = 0;
        let mut cancelled_jobs = 0;

        for uuid in &uuids {
            let post_id = transaction
                .query_row(
                    "SELECT id FROM posts WHERE uuid = ?1 AND deleted_at IS NULL",
                    params![uuid],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;

            let Some(post_id) = post_id else {
                continue;
            };

            let changed = transaction.execute(
                "UPDATE posts
                 SET deleted_at = CURRENT_TIMESTAMP,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![post_id],
            )?;

            if changed > 0 {
                deleted += changed;
                cancelled_jobs += transaction.execute(
                    "UPDATE job_queue
                     SET status = 'cancelled',
                         last_error = 'post bulk deleted',
                         updated_at = CURRENT_TIMESTAMP
                     WHERE kind = 'publish_post'
                       AND status = 'pending'
                       AND (
                            idempotency_key = ?1
                            OR (idempotency_key IS NULL AND payload_json = ?2)
                       )",
                    params![
                        publish_job_idempotency_key(post_id),
                        publish_job_payload(post_id)
                    ],
                )?;
            }
        }

        transaction.commit()?;

        Ok(BulkDeletePostsSummary {
            requested: uuids.len(),
            deleted,
            cancelled_jobs,
        })
    }

    pub fn services(&self) -> Result<Vec<ServiceSummary>, DbError> {
        let connection = self.connection()?;

        self.list_services(&connection, 100)
    }

    pub fn save_service(&self, form: &ServiceForm) -> Result<ServiceSummary, DbError> {
        let connection = self.connection()?;
        let service = form.validated().map_err(DbError::Validation)?;
        let name = service.name.clone();

        let configuration_json = service
            .configuration_json
            .clone()
            .unwrap_or_else(|| "{}".to_string());
        let should_update_configuration = service.configuration_json.is_some();

        connection.execute(
            "INSERT INTO services (name, configuration_secret_ref, configuration_json, active)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(name) DO UPDATE SET
                configuration_secret_ref = excluded.configuration_secret_ref,
                configuration_json = CASE
                    WHEN ?5 THEN excluded.configuration_json
                    ELSE services.configuration_json
                END,
                active = excluded.active",
            params![
                service.name,
                service.configuration_secret_ref,
                configuration_json,
                if service.active { 1 } else { 0 },
                should_update_configuration,
            ],
        )?;

        self.service_by_name(&connection, &name)
    }

    pub fn service_configuration_value(
        &self,
        service_name: &str,
        key: &str,
    ) -> Result<Option<String>, DbError> {
        let connection = self.connection()?;
        let configuration_json = connection
            .query_row(
                "SELECT configuration_json
                 FROM services
                 WHERE name = ?1",
                params![service_name],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(configuration_json) = configuration_json else {
            return Ok(None);
        };
        let configuration: serde_json::Value =
            serde_json::from_str(&configuration_json).map_err(|error| {
                DbError::Validation(format!("{service_name} configuration is invalid: {error}"))
            })?;

        Ok(configuration
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::to_string))
    }

    pub fn tags(&self) -> Result<Vec<TagSummary>, DbError> {
        let connection = self.connection()?;

        self.list_tags(&connection, 100)
    }

    pub fn create_tag(&self, form: &TagForm) -> Result<TagSummary, DbError> {
        let connection = self.connection()?;
        let tag = form.validated().map_err(DbError::Validation)?;
        let uuid = Uuid::new_v4().to_string();

        connection.execute(
            "INSERT INTO tags (uuid, name, hex_color, created_at, updated_at)
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            params![uuid, tag.name, tag.hex_color],
        )?;

        self.tag_by_uuid(&connection, &uuid)
    }

    pub fn update_tag(&self, uuid: &str, form: &TagForm) -> Result<TagSummary, DbError> {
        let connection = self.connection()?;
        let tag = form.validated().map_err(DbError::Validation)?;

        let changed = connection.execute(
            "UPDATE tags
             SET name = ?1, hex_color = ?2, updated_at = CURRENT_TIMESTAMP
             WHERE uuid = ?3",
            params![tag.name, tag.hex_color, uuid],
        )?;

        if changed == 0 {
            return Err(DbError::Validation("tag not found".to_string()));
        }

        self.tag_by_uuid(&connection, uuid)
    }

    pub fn delete_tag(&self, uuid: &str) -> Result<bool, DbError> {
        let connection = self.connection()?;
        let changed = connection.execute("DELETE FROM tags WHERE uuid = ?1", params![uuid])?;

        Ok(changed > 0)
    }

    pub fn enqueue_job(&self, form: &JobForm) -> Result<JobSummary, DbError> {
        let mut connection = self.connection()?;
        let job = form.validated().map_err(DbError::Validation)?;
        let transaction = connection.transaction()?;
        let existing = if let Some(idempotency_key) = job.idempotency_key.as_deref() {
            transaction
                .query_row(
                    "SELECT id, status
                     FROM job_queue
                     WHERE kind = ?1
                       AND idempotency_key = ?2
                       AND status IN ('pending', 'processing')
                     ORDER BY id ASC
                     LIMIT 1",
                    params![&job.kind, idempotency_key],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?
        } else {
            None
        };
        let job_id = if let Some((id, status)) = existing {
            if status == "pending" {
                transaction.execute(
                    "UPDATE job_queue
                     SET payload_json = ?2,
                         run_at = ?3,
                         last_error = NULL,
                         updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?1",
                    params![id, &job.payload_json, &job.run_at],
                )?;
            }

            id
        } else {
            transaction.execute(
                "INSERT INTO job_queue (
                    kind, payload_json, idempotency_key, status, attempts, run_at, created_at, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, 'pending', 0, ?4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
                 )",
                params![
                    &job.kind,
                    &job.payload_json,
                    &job.idempotency_key,
                    &job.run_at
                ],
            )?;

            transaction.last_insert_rowid()
        };

        transaction.commit()?;

        self.job_by_id(&connection, job_id)
    }

    pub fn reserve_due_jobs(&self, now: &str, limit: i64) -> Result<Vec<JobSummary>, DbError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let ids = {
            let mut statement = transaction.prepare(
                "SELECT id
                 FROM job_queue
                 WHERE status = 'pending' AND run_at <= ?1
                 ORDER BY run_at ASC, id ASC
                 LIMIT ?2",
            )?;

            let rows = statement.query_map(params![now, limit], |row| row.get::<_, i64>(0))?;

            collect_rows(rows)?
        };

        for id in &ids {
            transaction.execute(
                "UPDATE job_queue
                 SET status = 'processing',
                     attempts = attempts + 1,
                     locked_at = CURRENT_TIMESTAMP,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1 AND status = 'pending'",
                params![id],
            )?;
        }

        transaction.commit()?;

        self.jobs_by_ids(&connection, &ids)
    }

    #[cfg(test)]
    pub fn complete_job(&self, id: i64) -> Result<JobSummary, DbError> {
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE job_queue
             SET status = 'completed',
                 completed_at = CURRENT_TIMESTAMP,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![id],
        )?;

        if changed == 0 {
            return Err(DbError::Validation("job not found".to_string()));
        }

        self.job_by_id(&connection, id)
    }

    #[cfg(test)]
    pub fn fail_job(&self, id: i64, error: &str) -> Result<JobSummary, DbError> {
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE job_queue
             SET status = 'failed',
                 failed_at = CURRENT_TIMESTAMP,
                 last_error = ?2,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![id, error],
        )?;

        if changed == 0 {
            return Err(DbError::Validation("job not found".to_string()));
        }

        self.job_by_id(&connection, id)
    }

    pub fn retry_failed_account_import_jobs(
        &self,
        run_at: &str,
    ) -> Result<Vec<JobSummary>, DbError> {
        if run_at.trim().is_empty() {
            return Err(DbError::Validation("run_at is required".to_string()));
        }

        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let ids = {
            let mut statement = transaction.prepare(
                "SELECT id
                 FROM job_queue
                 WHERE kind = 'import_account_data'
                   AND status = 'failed'
                 ORDER BY failed_at ASC, id ASC",
            )?;
            let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;

            collect_rows(rows)?
        };

        for id in &ids {
            transaction.execute(
                "UPDATE job_queue
                 SET status = 'pending',
                     run_at = ?2,
                     locked_at = NULL,
                     last_error = NULL,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1
                   AND kind = 'import_account_data'
                   AND status = 'failed'",
                params![id, run_at],
            )?;
        }

        transaction.commit()?;

        self.jobs_by_ids(&connection, &ids)
    }

    #[cfg(test)]
    pub fn rate_limits(&self) -> Result<Vec<RateLimitSummary>, DbError> {
        let connection = self.connection()?;

        self.list_rate_limits(&connection, 100)
    }

    #[cfg(test)]
    pub fn save_rate_limit(&self, form: &RateLimitForm) -> Result<RateLimitSummary, DbError> {
        let connection = self.connection()?;
        let rate_limit = form.validated().map_err(DbError::Validation)?;
        let scope = rate_limit.scope.clone();

        connection.execute(
            "INSERT INTO rate_limits (
                scope, retry_after_at, payload_json, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
             )
             ON CONFLICT(scope) DO UPDATE SET
                retry_after_at = excluded.retry_after_at,
                payload_json = excluded.payload_json,
                updated_at = CURRENT_TIMESTAMP",
            params![
                rate_limit.scope,
                rate_limit.retry_after_at,
                rate_limit.payload_json
            ],
        )?;

        self.rate_limit_by_scope(&connection, &scope)
    }

    #[cfg(test)]
    pub fn clear_rate_limit(&self, scope: &str) -> Result<bool, DbError> {
        let connection = self.connection()?;
        let changed =
            connection.execute("DELETE FROM rate_limits WHERE scope = ?1", params![scope])?;

        Ok(changed > 0)
    }

    pub fn run_due_jobs(&self, now: &str, limit: i64) -> Result<WorkerRunSummary, DbError> {
        if now.trim().is_empty() {
            return Err(DbError::Validation("now is required".to_string()));
        }

        let limit = limit.clamp(1, 100);
        let reserved = self.reserve_due_jobs(now, limit)?;
        let mut outcomes = Vec::with_capacity(reserved.len());

        for job in &reserved {
            outcomes.push(self.run_reserved_job(job, now)?);
        }

        let completed = outcomes
            .iter()
            .filter(|outcome| outcome.status == "completed")
            .count();
        let failed = outcomes
            .iter()
            .filter(|outcome| outcome.status == "failed")
            .count();
        let deferred = outcomes
            .iter()
            .filter(|outcome| outcome.status == "deferred")
            .count();

        Ok(WorkerRunSummary {
            now: now.to_string(),
            limit,
            reserved: reserved.len(),
            completed,
            deferred,
            failed,
            outcomes,
        })
    }

    fn run_reserved_job(&self, job: &JobSummary, now: &str) -> Result<WorkerJobOutcome, DbError> {
        match job.kind.as_str() {
            "publish_post" => self.run_publish_post_job(job, now),
            "import_account_data" => self.run_import_account_job(job),
            _ => self.fail_reserved_job(job, &format!("unsupported job kind: {}", job.kind)),
        }
    }

    fn run_import_account_job(&self, job: &JobSummary) -> Result<WorkerJobOutcome, DbError> {
        let payload = match self.job_payload(job.id) {
            Ok(payload) => payload,
            Err(error) => {
                return self.fail_reserved_job(job, &format!("invalid import payload: {error}"));
            }
        };
        let account_id = match payload
            .get("account_id")
            .and_then(serde_json::Value::as_i64)
        {
            Some(account_id) if account_id > 0 => account_id,
            _ => {
                return self
                    .fail_reserved_job(job, "import payload must contain a positive account_id");
            }
        };
        let (uuid, provider) = match self.import_job_account(account_id) {
            Ok(account) => account,
            Err(error) => return self.fail_reserved_job(job, &error.to_string()),
        };
        let detail = match provider.as_str() {
            "twitter" => match self.import_twitter_account_data(&uuid) {
                Ok(summary) => format!(
                    "imported X account {}: {} post(s), {} metric day(s)",
                    summary.account.name, summary.imported_posts, summary.metric_days
                ),
                Err(error) => return self.fail_reserved_job(job, &error.to_string()),
            },
            "facebook_page" => match self.import_facebook_page_data(&uuid) {
                Ok(summary) => format!(
                    "imported Facebook Page {}: {} insight row(s)",
                    summary.account.name, summary.insight_rows
                ),
                Err(error) => return self.fail_reserved_job(job, &error.to_string()),
            },
            "mastodon" => match self.import_mastodon_account_data(&uuid) {
                Ok(summary) => format!(
                    "imported Mastodon account {}: {} post(s), {} metric day(s)",
                    summary.account.name, summary.imported_posts, summary.metric_days
                ),
                Err(error) => return self.fail_reserved_job(job, &error.to_string()),
            },
            _ => {
                return self
                    .fail_reserved_job(job, &format!("{provider} imports are not supported yet"));
            }
        };

        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;

        Self::mark_reserved_job_completed(&transaction, job.id)?;
        transaction.commit()?;

        Ok(WorkerJobOutcome {
            job_id: job.id,
            kind: job.kind.clone(),
            status: "completed".to_string(),
            detail,
        })
    }

    fn job_payload(&self, job_id: i64) -> Result<serde_json::Value, DbError> {
        let connection = self.connection()?;
        let payload_json: String = connection.query_row(
            "SELECT payload_json FROM job_queue WHERE id = ?1",
            params![job_id],
            |row| row.get(0),
        )?;

        serde_json::from_str(&payload_json).map_err(DbError::Json)
    }

    fn fail_reserved_job(
        &self,
        job: &JobSummary,
        detail: &str,
    ) -> Result<WorkerJobOutcome, DbError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;

        Self::mark_reserved_job_failed(&transaction, job.id, detail)?;
        transaction.commit()?;

        Ok(WorkerJobOutcome {
            job_id: job.id,
            kind: job.kind.clone(),
            status: "failed".to_string(),
            detail: detail.to_string(),
        })
    }

    fn run_publish_post_job(
        &self,
        job: &JobSummary,
        now: &str,
    ) -> Result<WorkerJobOutcome, DbError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let payload_json: String = transaction
            .query_row(
                "SELECT payload_json
                 FROM job_queue
                 WHERE id = ?1 AND status = 'processing'",
                params![job.id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| DbError::Validation("reserved job not found".to_string()))?;

        let payload = match serde_json::from_str::<serde_json::Value>(&payload_json) {
            Ok(payload) => payload,
            Err(error) => {
                let detail = format!("invalid publish payload: {error}");
                Self::mark_reserved_job_failed(&transaction, job.id, &detail)?;
                transaction.commit()?;

                return Ok(WorkerJobOutcome {
                    job_id: job.id,
                    kind: job.kind.clone(),
                    status: "failed".to_string(),
                    detail,
                });
            }
        };

        let post_id = match payload.get("post_id").and_then(serde_json::Value::as_i64) {
            Some(post_id) if post_id > 0 => post_id,
            _ => {
                let detail = "publish payload must contain a positive post_id".to_string();
                Self::mark_reserved_job_failed(&transaction, job.id, &detail)?;
                transaction.commit()?;

                return Ok(WorkerJobOutcome {
                    job_id: job.id,
                    kind: job.kind.clone(),
                    status: "failed".to_string(),
                    detail,
                });
            }
        };

        let post_state: Option<(i64, i64)> = transaction
            .query_row(
                "SELECT status, schedule_status
                 FROM posts
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![post_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        let Some((status, _schedule_status)) = post_state else {
            let detail = format!("post {post_id} not found");
            Self::mark_reserved_job_failed(&transaction, job.id, &detail)?;
            transaction.commit()?;

            return Ok(WorkerJobOutcome {
                job_id: job.id,
                kind: job.kind.clone(),
                status: "failed".to_string(),
                detail,
            });
        };

        if matches!(status, 2 | 3) {
            let detail = if status == 2 {
                format!("post {post_id} already published")
            } else {
                format!("post {post_id} already failed")
            };

            Self::mark_reserved_job_completed(&transaction, job.id)?;
            transaction.commit()?;

            return Ok(WorkerJobOutcome {
                job_id: job.id,
                kind: job.kind.clone(),
                status: "completed".to_string(),
                detail,
            });
        }

        if status != 1 {
            let detail = format!("post {post_id} is not scheduled");
            Self::mark_reserved_job_failed(&transaction, job.id, &detail)?;
            transaction.commit()?;

            return Ok(WorkerJobOutcome {
                job_id: job.id,
                kind: job.kind.clone(),
                status: "failed".to_string(),
                detail,
            });
        }

        let validation = self.validate_post_for_publish(&transaction, post_id)?;
        if !validation.valid {
            let detail = format_validation_errors(&validation);
            Self::insert_validation_errors(&transaction, post_id, &validation)?;
            transaction.execute(
                "UPDATE posts
                 SET status = 3,
                     schedule_status = 2,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![post_id],
            )?;
            Self::mark_reserved_job_failed(&transaction, job.id, &detail)?;
            transaction.commit()?;

            return Ok(WorkerJobOutcome {
                job_id: job.id,
                kind: job.kind.clone(),
                status: "failed".to_string(),
                detail,
            });
        }

        if let Some(block) = Self::active_publish_rate_limit(&transaction, post_id, now)? {
            let detail = format!(
                "publish deferred by rate limit {} until {}",
                block.scope, block.retry_after_at
            );

            Self::mark_reserved_job_deferred(&transaction, job.id, &block.retry_after_at, &detail)?;
            transaction.commit()?;

            return Ok(WorkerJobOutcome {
                job_id: job.id,
                kind: job.kind.clone(),
                status: "deferred".to_string(),
                detail,
            });
        }

        transaction.execute(
            "UPDATE posts
             SET schedule_status = 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![post_id],
        )?;

        let account_count: i64 = transaction.query_row(
            "SELECT COUNT(*)
             FROM post_accounts
             WHERE post_id = ?1",
            params![post_id],
            |row| row.get(0),
        )?;

        if account_count == 0 {
            let detail = format!("post {post_id} has no target accounts");

            transaction.execute(
                "UPDATE posts
                 SET status = 3,
                     schedule_status = 2,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![post_id],
            )?;
            Self::mark_reserved_job_failed(&transaction, job.id, &detail)?;
            transaction.commit()?;

            return Ok(WorkerJobOutcome {
                job_id: job.id,
                kind: job.kind.clone(),
                status: "failed".to_string(),
                detail,
            });
        }

        let unauthorized_accounts = {
            let mut statement = transaction.prepare(
                "SELECT COALESCE(a.username, a.name, a.provider_id)
                 FROM post_accounts pa
                 INNER JOIN accounts a ON a.id = pa.account_id
                 WHERE pa.post_id = ?1 AND a.authorized = 0
                 ORDER BY a.id ASC",
            )?;
            let rows = statement.query_map(params![post_id], |row| row.get::<_, String>(0))?;

            collect_rows(rows)?
        };

        if !unauthorized_accounts.is_empty() {
            let account_error_json = serde_json::to_string(&vec!["Access token expired"])?;
            let detail = format!(
                "post {post_id} failed because {} account(s) need authorization: {}",
                unauthorized_accounts.len(),
                unauthorized_accounts.join(", ")
            );

            transaction.execute(
                "UPDATE post_accounts
                 SET errors_json = ?2
                 WHERE post_id = ?1
                   AND account_id IN (
                        SELECT id FROM accounts WHERE authorized = 0
                   )",
                params![post_id, account_error_json],
            )?;
            transaction.execute(
                "UPDATE posts
                 SET status = 3,
                     schedule_status = 2,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![post_id],
            )?;
            Self::mark_reserved_job_failed(&transaction, job.id, &detail)?;
            transaction.commit()?;

            return Ok(WorkerJobOutcome {
                job_id: job.id,
                kind: job.kind.clone(),
                status: "failed".to_string(),
                detail,
            });
        }

        let targets = self.publish_targets(&transaction, post_id)?;
        transaction.commit()?;
        let batch = self.publish_post_targets(&targets, now)?;

        self.finish_publish_post_job(job, post_id, now, batch)
    }

    fn publish_targets(
        &self,
        connection: &Connection,
        post_id: i64,
    ) -> Result<Vec<PublishTarget>, DbError> {
        let versions = self.post_versions(connection, post_id)?;
        let original_content = versions
            .iter()
            .find(|version| version.is_original)
            .or_else(|| versions.first())
            .and_then(|version| version.content.first())
            .cloned();
        let mut statement = connection.prepare(
            "SELECT pa.id, a.id, a.provider, a.provider_id, a.data_json
             FROM post_accounts pa
             INNER JOIN accounts a ON a.id = pa.account_id
             WHERE pa.post_id = ?1
             ORDER BY pa.id ASC",
        )?;
        let rows = statement.query_map(params![post_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;
        let account_rows = collect_rows(rows)?;
        let mut targets = Vec::with_capacity(account_rows.len());

        for (post_account_id, account_id, provider, provider_id, account_data_json) in account_rows
        {
            let content = versions
                .iter()
                .find(|version| version.account_id == account_id)
                .and_then(|version| version.content.first())
                .cloned()
                .or_else(|| original_content.clone())
                .ok_or_else(|| {
                    DbError::Validation(format!(
                        "post {post_id} has no publishable content for account {account_id}"
                    ))
                })?;

            targets.push(PublishTarget {
                post_account_id,
                account_id,
                provider,
                provider_id,
                account_data_json,
                content,
            });
        }

        Ok(targets)
    }

    fn publish_post_targets(
        &self,
        targets: &[PublishTarget],
        now: &str,
    ) -> Result<PublishBatchResult, DbError> {
        let mut batch = PublishBatchResult::default();

        for target in targets {
            let result = self.publish_account_target(target, now)?;

            if result.published_remotely {
                batch.remote_count += 1;
            }

            if let Some(detail) = result.error_detail.as_ref() {
                batch.failures.push(detail.clone());
            }

            batch.results.push(result);
        }

        Ok(batch)
    }

    fn publish_account_target(
        &self,
        target: &PublishTarget,
        now: &str,
    ) -> Result<PublishAccountResult, DbError> {
        match target.provider.as_str() {
            "mastodon" => {
                if let Some(server) =
                    mastodon_server_from_account_data(target.account_data_json.as_deref())?
                {
                    return self.publish_mastodon_target(target, &server, now);
                }

                failed_publish_result(
                    target,
                    "Mastodon account must be connected before publishing",
                )
            }
            "twitter" => {
                if twitter_oauth2_account_data(target.account_data_json.as_deref())? {
                    return self.publish_twitter_target(target, now);
                }

                failed_publish_result(target, "X account must be connected before publishing")
            }
            "facebook_page" => {
                if facebook_page_connected_account_data(target.account_data_json.as_deref())? {
                    return self.publish_facebook_page_target(target, now);
                }

                failed_publish_result(
                    target,
                    "Facebook Page account must be connected before publishing",
                )
            }
            provider => failed_publish_result(
                target,
                &format!("{provider} publishing is not supported yet"),
            ),
        }
    }

    fn publish_media_assets(
        &self,
        content: &PostContentBlock,
    ) -> Result<Vec<PublishMediaAsset>, DbError> {
        if content.media.is_empty() && content.external_media.is_empty() {
            return Ok(Vec::new());
        }

        let connection = self.connection()?;
        let mut assets = Vec::with_capacity(content.media.len() + content.external_media.len());

        for media_id in &content.media {
            let media: Option<(String, String, String)> = connection
                .query_row(
                    "SELECT disk, path, mime_type
                     FROM media
                     WHERE id = ?1",
                    params![media_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?;
            let Some((disk, path, mime_type)) = media else {
                return Err(DbError::Validation(format!(
                    "media {media_id} is missing and cannot be published"
                )));
            };
            let Some(resource_path) = self.media_resource_path(&disk, &path) else {
                return Err(DbError::Validation(format!(
                    "media {media_id} is not available as a local publishable file"
                )));
            };

            if resource_path.starts_with("http://") || resource_path.starts_with("https://") {
                return Err(DbError::Validation(format!(
                    "media {media_id} must be imported into local storage before publishing"
                )));
            }

            let file_path = PathBuf::from(resource_path);

            if !file_path.exists() {
                return Err(DbError::Validation(format!(
                    "media {media_id} file not found: {}",
                    file_path.display()
                )));
            }

            assets.push(PublishMediaAsset {
                id: media_id.to_string(),
                file_path,
                mime_type,
                temporary: false,
            });
        }

        for external_media in &content.external_media {
            match self.transient_external_media_asset(external_media) {
                Ok(asset) => assets.push(asset),
                Err(error) => {
                    cleanup_publish_assets(&assets);
                    return Err(error);
                }
            }
        }

        Ok(assets)
    }

    fn publish_mastodon_target(
        &self,
        target: &PublishTarget,
        server: &str,
        now: &str,
    ) -> Result<PublishAccountResult, DbError> {
        let status = validation_text(&target.content.body);
        let access_token =
            match resolve_account_secret("mastodon", &target.provider_id, "access_token") {
                Ok(value) => value,
                Err(error) => return failed_publish_result(target, &error.to_string()),
            };
        let mut media_ids = Vec::new();
        let mut uploaded_media = Vec::new();

        let media_assets = self.publish_media_assets(&target.content)?;

        for media in &media_assets {
            let upload_result = upload_mastodon_media(&MastodonMediaUploadRequest {
                server: server.to_string(),
                access_token: access_token.clone(),
                file_path: media.file_path.clone(),
                mime_type: media.mime_type.clone(),
            });

            let response = match upload_result {
                Ok(response) => response,
                Err(error) => {
                    cleanup_publish_assets(&media_assets);
                    return failed_publish_result(
                        target,
                        &format!("media {} upload failed: {error}", media.id),
                    );
                }
            };

            media_ids.push(response.id);
            uploaded_media.push(response.raw);
        }
        cleanup_publish_assets(&media_assets);

        let response = match publish_mastodon_status(&MastodonPublishRequest {
            server: server.to_string(),
            access_token,
            status,
            media_ids,
        }) {
            Ok(response) => response,
            Err(error) => return failed_publish_result(target, &error.to_string()),
        };
        let data_json = serde_json::json!({
            "provider": "mastodon",
            "published_at": now,
            "url": response.url,
            "uploaded_media": uploaded_media,
            "raw": response.raw
        })
        .to_string();

        Ok(PublishAccountResult {
            post_account_id: target.post_account_id,
            provider_post_id: Some(response.id),
            data_json: Some(data_json),
            errors_json: None,
            error_detail: None,
            published_remotely: true,
        })
    }

    fn publish_twitter_target(
        &self,
        target: &PublishTarget,
        now: &str,
    ) -> Result<PublishAccountResult, DbError> {
        let text = validation_text(&target.content.body);
        let access_token =
            match resolve_account_secret("twitter", &target.provider_id, "access_token") {
                Ok(value) => value,
                Err(error) => return failed_publish_result(target, &error.to_string()),
            };
        let mut media_ids = Vec::new();
        let mut uploaded_media = Vec::new();

        let media_assets = self.publish_media_assets(&target.content)?;

        for media in &media_assets {
            let upload_result = upload_twitter_media(&TwitterMediaUploadRequest {
                access_token: access_token.clone(),
                file_path: media.file_path.clone(),
                mime_type: media.mime_type.clone(),
            });

            let response = match upload_result {
                Ok(response) => response,
                Err(error) => {
                    cleanup_publish_assets(&media_assets);
                    return failed_publish_result(
                        target,
                        &format!("media {} upload failed: {error}", media.id),
                    );
                }
            };

            media_ids.push(response.id);
            uploaded_media.push(response.raw);
        }
        cleanup_publish_assets(&media_assets);

        let response = match publish_twitter_post(&TwitterPublishRequest {
            access_token,
            text,
            media_ids,
        }) {
            Ok(response) => response,
            Err(error) => return failed_publish_result(target, &error.to_string()),
        };
        let data_json = serde_json::json!({
            "provider": "twitter",
            "published_at": now,
            "uploaded_media": uploaded_media,
            "raw": response.raw
        })
        .to_string();

        Ok(PublishAccountResult {
            post_account_id: target.post_account_id,
            provider_post_id: Some(response.id),
            data_json: Some(data_json),
            errors_json: None,
            error_detail: None,
            published_remotely: true,
        })
    }

    fn publish_facebook_page_target(
        &self,
        target: &PublishTarget,
        now: &str,
    ) -> Result<PublishAccountResult, DbError> {
        let text = validation_text(&target.content.body);
        let page_access_token =
            match resolve_account_secret("facebook_page", &target.provider_id, "page_access_token")
            {
                Ok(value) => value,
                Err(error) => return failed_publish_result(target, &error.to_string()),
            };
        let api_version = self.service_configuration_value("facebook", "api_version")?;
        let media_assets = self.publish_media_assets(&target.content)?;

        if media_assets
            .first()
            .is_some_and(|media| media.mime_type.starts_with("video/"))
        {
            let media = media_assets
                .first()
                .expect("first media exists after is_some_and");
            let upload_result = upload_facebook_page_video(&FacebookPageVideoUploadRequest {
                page_id: target.provider_id.clone(),
                page_access_token: page_access_token.clone(),
                file_path: media.file_path.clone(),
                mime_type: media.mime_type.clone(),
                description: text,
                api_version: api_version.clone(),
            });
            cleanup_publish_assets(&media_assets);

            let response = match upload_result {
                Ok(response) => response,
                Err(error) => {
                    return failed_publish_result(
                        target,
                        &format!("media {} upload failed: {error}", media.id),
                    );
                }
            };
            let data_json = serde_json::json!({
                "provider": "facebook_page",
                "published_at": now,
                "uploaded_video": response.raw
            })
            .to_string();

            return Ok(PublishAccountResult {
                post_account_id: target.post_account_id,
                provider_post_id: Some(response.id),
                data_json: Some(data_json),
                errors_json: None,
                error_detail: None,
                published_remotely: true,
            });
        }

        let mut media_ids = Vec::new();
        let mut uploaded_media = Vec::new();

        for media in &media_assets {
            let upload_result = upload_facebook_page_photo(&FacebookPagePhotoUploadRequest {
                page_id: target.provider_id.clone(),
                page_access_token: page_access_token.clone(),
                file_path: media.file_path.clone(),
                mime_type: media.mime_type.clone(),
                api_version: api_version.clone(),
            });

            let response = match upload_result {
                Ok(response) => response,
                Err(error) => {
                    cleanup_publish_assets(&media_assets);
                    return failed_publish_result(
                        target,
                        &format!("media {} upload failed: {error}", media.id),
                    );
                }
            };

            media_ids.push(response.id);
            uploaded_media.push(response.raw);
        }
        cleanup_publish_assets(&media_assets);

        let response = match publish_facebook_page_post(&FacebookPagePublishRequest {
            page_id: target.provider_id.clone(),
            page_access_token,
            text,
            media_ids,
            api_version,
        }) {
            Ok(response) => response,
            Err(error) => return failed_publish_result(target, &error.to_string()),
        };
        let data_json = serde_json::json!({
            "provider": "facebook_page",
            "published_at": now,
            "uploaded_media": uploaded_media,
            "raw": response.raw
        })
        .to_string();

        Ok(PublishAccountResult {
            post_account_id: target.post_account_id,
            provider_post_id: Some(response.id),
            data_json: Some(data_json),
            errors_json: None,
            error_detail: None,
            published_remotely: true,
        })
    }

    fn finish_publish_post_job(
        &self,
        job: &JobSummary,
        post_id: i64,
        now: &str,
        batch: PublishBatchResult,
    ) -> Result<WorkerJobOutcome, DbError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;

        for result in &batch.results {
            transaction.execute(
                "UPDATE post_accounts
                 SET provider_post_id = COALESCE(?3, provider_post_id),
                     data_json = COALESCE(?4, data_json),
                     errors_json = ?5
                 WHERE id = ?1 AND post_id = ?2",
                params![
                    result.post_account_id,
                    post_id,
                    result.provider_post_id,
                    result.data_json,
                    result.errors_json
                ],
            )?;
        }

        if batch.failures.is_empty() {
            transaction.execute(
                "UPDATE posts
                 SET status = 2,
                     schedule_status = 2,
                     published_at = ?2,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![post_id, now],
            )?;
            Self::mark_reserved_job_completed(&transaction, job.id)?;
            transaction.commit()?;

            return Ok(WorkerJobOutcome {
                job_id: job.id,
                kind: job.kind.clone(),
                status: "completed".to_string(),
                detail: publish_success_detail(post_id, batch.remote_count),
            });
        }

        let detail = format!(
            "post {post_id} failed for {} account(s): {}",
            batch.failures.len(),
            batch.failures.join("; ")
        );

        transaction.execute(
            "UPDATE posts
             SET status = 3,
                 schedule_status = 2,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![post_id],
        )?;
        Self::mark_reserved_job_failed(&transaction, job.id, &detail)?;
        transaction.commit()?;

        Ok(WorkerJobOutcome {
            job_id: job.id,
            kind: job.kind.clone(),
            status: "failed".to_string(),
            detail,
        })
    }

    fn mark_reserved_job_completed(transaction: &Transaction<'_>, id: i64) -> Result<(), DbError> {
        let changed = transaction.execute(
            "UPDATE job_queue
             SET status = 'completed',
                 completed_at = CURRENT_TIMESTAMP,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1 AND status = 'processing'",
            params![id],
        )?;

        if changed == 0 {
            return Err(DbError::Validation("reserved job not found".to_string()));
        }

        Ok(())
    }

    fn mark_reserved_job_failed(
        transaction: &Transaction<'_>,
        id: i64,
        error: &str,
    ) -> Result<(), DbError> {
        let changed = transaction.execute(
            "UPDATE job_queue
             SET status = 'failed',
                 failed_at = CURRENT_TIMESTAMP,
                 last_error = ?2,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1 AND status = 'processing'",
            params![id, error],
        )?;

        if changed == 0 {
            return Err(DbError::Validation("reserved job not found".to_string()));
        }

        Ok(())
    }

    fn mark_reserved_job_deferred(
        transaction: &Transaction<'_>,
        id: i64,
        run_at: &str,
        detail: &str,
    ) -> Result<(), DbError> {
        let changed = transaction.execute(
            "UPDATE job_queue
             SET status = 'pending',
                 run_at = ?2,
                 locked_at = NULL,
                 last_error = ?3,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1 AND status = 'processing'",
            params![id, run_at, detail],
        )?;

        if changed == 0 {
            return Err(DbError::Validation("reserved job not found".to_string()));
        }

        Ok(())
    }

    fn active_publish_rate_limit(
        transaction: &Transaction<'_>,
        post_id: i64,
        now: &str,
    ) -> Result<Option<RateLimitBlock>, DbError> {
        let accounts = {
            let mut statement = transaction.prepare(
                "SELECT a.id, a.provider
                 FROM post_accounts pa
                 INNER JOIN accounts a ON a.id = pa.account_id
                 WHERE pa.post_id = ?1
                 ORDER BY a.id ASC",
            )?;
            let rows = statement.query_map(params![post_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?;

            collect_rows(rows)?
        };

        for (account_id, provider) in accounts {
            if let Some(block) = Self::active_rate_limit_by_scope(
                transaction,
                &app_rate_limit_scope(&provider),
                now,
            )? {
                return Ok(Some(block));
            }

            if let Some(block) = Self::active_rate_limit_by_scope(
                transaction,
                &account_rate_limit_scope(account_id),
                now,
            )? {
                return Ok(Some(block));
            }
        }

        Ok(None)
    }

    fn active_rate_limit_by_scope(
        transaction: &Transaction<'_>,
        scope: &str,
        now: &str,
    ) -> Result<Option<RateLimitBlock>, DbError> {
        transaction
            .query_row(
                "SELECT scope, retry_after_at
                 FROM rate_limits
                 WHERE scope = ?1 AND retry_after_at > ?2",
                params![scope, now],
                |row| {
                    Ok(RateLimitBlock {
                        scope: row.get(0)?,
                        retry_after_at: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(DbError::Sqlite)
    }

    fn account_report_for_end_date(
        &self,
        account_id: i64,
        period: &str,
        days: i64,
        end_date: NaiveDate,
    ) -> Result<ReportSnapshot, DbError> {
        let connection = self.connection()?;
        let provider: String = connection.query_row(
            "SELECT provider FROM accounts WHERE id = ?1",
            params![account_id],
            |row| row.get(0),
        )?;
        let start_date = end_date - Duration::days(days - 1);
        let start = start_date.to_string();
        let end = end_date.to_string();

        Ok(ReportSnapshot {
            account_id,
            provider: provider.clone(),
            period: period.to_string(),
            tier: if provider == "twitter" {
                Some(
                    self.service_configuration_value("twitter", "tier")?
                        .unwrap_or_else(|| "legacy".to_string()),
                )
            } else {
                None
            },
            metrics: self.report_metrics(&connection, account_id, &provider, &start, &end)?,
            audience: self.report_audience(
                &connection,
                account_id,
                days,
                start_date,
                &start,
                &end,
            )?,
        })
    }

    fn report_metrics(
        &self,
        connection: &Connection,
        account_id: i64,
        provider: &str,
        start: &str,
        end: &str,
    ) -> Result<Vec<ReportMetric>, DbError> {
        match provider {
            "twitter" => self.metric_json_sums(
                connection,
                account_id,
                start,
                end,
                &["likes", "retweets", "impressions"],
            ),
            "mastodon" => self.metric_json_sums(
                connection,
                account_id,
                start,
                end,
                &["replies", "reblogs", "favourites"],
            ),
            "facebook_page" => self.facebook_insight_sums(connection, account_id, start, end),
            _ => Ok(Vec::new()),
        }
    }

    fn metric_json_sums(
        &self,
        connection: &Connection,
        account_id: i64,
        start: &str,
        end: &str,
        keys: &[&str],
    ) -> Result<Vec<ReportMetric>, DbError> {
        let mut sums = keys
            .iter()
            .map(|key| ((*key).to_string(), 0_i64))
            .collect::<BTreeMap<_, _>>();
        let mut statement = connection.prepare(
            "SELECT data_json
             FROM metrics
             WHERE account_id = ?1 AND date >= ?2 AND date <= ?3",
        )?;
        let rows = statement.query_map(params![account_id, start, end], |row| {
            row.get::<_, String>(0)
        })?;
        let payloads = collect_rows(rows)?;

        for payload in payloads {
            let value = serde_json::from_str::<serde_json::Value>(&payload)?;

            for key in keys {
                if let Some(total) = value.get(*key).and_then(json_number_to_i64) {
                    if let Some(sum) = sums.get_mut(*key) {
                        *sum += total;
                    }
                }
            }
        }

        Ok(keys
            .iter()
            .map(|key| ReportMetric {
                key: (*key).to_string(),
                value: *sums.get(*key).unwrap_or(&0),
            })
            .collect())
    }

    fn facebook_insight_sums(
        &self,
        connection: &Connection,
        account_id: i64,
        start: &str,
        end: &str,
    ) -> Result<Vec<ReportMetric>, DbError> {
        let mut sums = BTreeMap::from([
            ("page_post_engagements".to_string(), 0_i64),
            ("page_posts_impressions".to_string(), 0_i64),
        ]);
        let mut statement = connection.prepare(
            "SELECT type, SUM(value)
             FROM facebook_insights
             WHERE account_id = ?1 AND date >= ?2 AND date <= ?3
             GROUP BY type",
        )?;
        let rows = statement.query_map(params![account_id, start, end], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?;

        for row in collect_rows(rows)? {
            match row {
                (2, total) => {
                    sums.insert("page_post_engagements".to_string(), total);
                }
                (3, total) => {
                    sums.insert("page_posts_impressions".to_string(), total);
                }
                _ => {}
            }
        }

        Ok(["page_post_engagements", "page_posts_impressions"]
            .into_iter()
            .map(|key| ReportMetric {
                key: key.to_string(),
                value: *sums.get(key).unwrap_or(&0),
            })
            .collect())
    }

    fn report_audience(
        &self,
        connection: &Connection,
        account_id: i64,
        days: i64,
        start_date: NaiveDate,
        start: &str,
        end: &str,
    ) -> Result<AudienceReport, DbError> {
        let mut statement = connection.prepare(
            "SELECT date, SUM(total)
             FROM audience
             WHERE account_id = ?1 AND date >= ?2 AND date <= ?3
             GROUP BY date
             ORDER BY date ASC",
        )?;
        let rows = statement.query_map(params![account_id, start, end], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let totals = collect_rows(rows)?
            .into_iter()
            .collect::<BTreeMap<String, i64>>();
        let first_date = totals.keys().next().cloned();
        let mut labels = Vec::with_capacity(days as usize);
        let mut values = Vec::with_capacity(days as usize);
        let mut points = Vec::with_capacity(days as usize);

        for offset in 0..days {
            let date = start_date + Duration::days(offset);
            let date_string = date.to_string();
            let label = date.format("%b %-d").to_string();
            let value = if first_date
                .as_ref()
                .is_some_and(|first| date_string >= *first)
            {
                totals.get(&date_string).copied()
            } else {
                None
            };

            labels.push(label.clone());
            values.push(value);
            points.push(AudiencePoint {
                date: date_string,
                label,
                value,
            });
        }

        Ok(AudienceReport {
            labels,
            values,
            points,
        })
    }

    fn validate_post_for_publish(
        &self,
        connection: &Connection,
        post_id: i64,
    ) -> Result<PostValidationReport, DbError> {
        let capabilities = provider_capabilities()
            .into_iter()
            .map(|capability| (capability.id, capability))
            .collect::<BTreeMap<_, _>>();
        let versions = self.post_versions(connection, post_id)?;
        let original = versions
            .iter()
            .find(|version| version.is_original)
            .or_else(|| versions.first());
        let mut errors = Vec::new();
        let accounts = {
            let mut statement = connection.prepare(
                "SELECT a.id, a.provider, COALESCE(a.username, a.name, a.provider_id)
                 FROM post_accounts pa
                 INNER JOIN accounts a ON a.id = pa.account_id
                 WHERE pa.post_id = ?1
                 ORDER BY pa.id ASC",
            )?;
            let rows = statement.query_map(params![post_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;

            collect_rows(rows)?
        };

        if accounts.is_empty() {
            errors.push(PostValidationError {
                account_id: 0,
                provider: "unknown".to_string(),
                code: "no_accounts".to_string(),
                message: "post must have at least one account before publishing".to_string(),
            });
        }

        let mut provider_account_counts = BTreeMap::new();

        for (_account_id, provider, _account_name) in &accounts {
            *provider_account_counts
                .entry(provider.as_str())
                .or_insert(0usize) += 1;
        }

        for (provider, count) in provider_account_counts {
            let Some(capability) = capabilities.get(provider) else {
                continue;
            };

            if !capability.post_config.simultaneous_posting && count > 1 {
                errors.push(PostValidationError {
                    account_id: 0,
                    provider: provider.to_string(),
                    code: "simultaneous_posting_disabled".to_string(),
                    message: format!(
                        "{} does not allow simultaneous posting to multiple accounts",
                        capability.display_name
                    ),
                });
            }
        }

        for (account_id, provider, account_name) in accounts {
            let Some(capability) = capabilities.get(provider.as_str()) else {
                errors.push(PostValidationError {
                    account_id,
                    provider,
                    code: "unsupported_provider".to_string(),
                    message: "provider has no local publish capability contract".to_string(),
                });
                continue;
            };

            let content = versions
                .iter()
                .find(|version| version.account_id == account_id)
                .or(original)
                .and_then(|version| version.content.first());

            let Some(content) = content else {
                errors.push(PostValidationError {
                    account_id,
                    provider: provider.clone(),
                    code: "no_content".to_string(),
                    message: format!("{account_name} has no post content"),
                });
                continue;
            };

            let body = validation_text(&content.body);
            let text_len = body.chars().count();
            let media_counts = self.media_kind_counts(connection, content)?;

            if body.is_empty() && content.media.is_empty() && content.external_media.is_empty() {
                errors.push(PostValidationError {
                    account_id,
                    provider: provider.clone(),
                    code: "no_content".to_string(),
                    message: format!("{account_name} needs text or media"),
                });
            }

            if text_len > usize::from(capability.post_config.max_text_chars) {
                errors.push(PostValidationError {
                    account_id,
                    provider: provider.clone(),
                    code: "text_too_long".to_string(),
                    message: format!(
                        "{account_name} text has {text_len} characters; {} allows {}",
                        capability.display_name, capability.post_config.max_text_chars
                    ),
                });
            }

            if media_counts.missing > 0 {
                errors.push(PostValidationError {
                    account_id,
                    provider: provider.clone(),
                    code: "media_missing".to_string(),
                    message: format!(
                        "{account_name} references {} missing media item(s)",
                        media_counts.missing
                    ),
                });
            }

            Self::validate_media_count(
                &mut errors,
                account_id,
                &provider,
                &account_name,
                capability,
                "photo",
                media_counts.photos,
                capability.post_config.max_media.photos,
            );
            Self::validate_media_count(
                &mut errors,
                account_id,
                &provider,
                &account_name,
                capability,
                "video",
                media_counts.videos,
                capability.post_config.max_media.videos,
            );
            Self::validate_media_count(
                &mut errors,
                account_id,
                &provider,
                &account_name,
                capability,
                "gif",
                media_counts.gifs,
                capability.post_config.max_media.gifs,
            );

            if !capability.post_config.max_media.allow_mixing && media_counts.mixed() {
                errors.push(PostValidationError {
                    account_id,
                    provider: provider.clone(),
                    code: "mixed_media".to_string(),
                    message: format!(
                        "{} does not allow mixed media types",
                        capability.display_name
                    ),
                });
            }
        }

        Ok(PostValidationReport {
            valid: errors.is_empty(),
            errors,
        })
    }

    fn validate_media_count(
        errors: &mut Vec<PostValidationError>,
        account_id: i64,
        provider: &str,
        account_name: &str,
        capability: &ProviderCapability,
        media_type: &str,
        used: usize,
        limit: u8,
    ) {
        if used <= usize::from(limit) {
            return;
        }

        errors.push(PostValidationError {
            account_id,
            provider: provider.to_string(),
            code: format!("too_many_{media_type}s"),
            message: format!(
                "{account_name} has {used} {media_type}(s); {} allows {limit}",
                capability.display_name
            ),
        });
    }

    fn media_kind_counts(
        &self,
        connection: &Connection,
        content: &PostContentBlock,
    ) -> Result<MediaKindCounts, DbError> {
        let mut counts = MediaKindCounts::default();

        for media_id in &content.media {
            let mime_type: Option<String> = connection
                .query_row(
                    "SELECT mime_type FROM media WHERE id = ?1",
                    params![media_id],
                    |row| row.get(0),
                )
                .optional()?;

            match mime_type.as_deref() {
                Some("image/gif") => counts.gifs += 1,
                Some(value) if value.starts_with("image/") => counts.photos += 1,
                Some(value) if value.starts_with("video/") => counts.videos += 1,
                Some(_) => counts.photos += 1,
                None => counts.missing += 1,
            }
        }

        for external_media in &content.external_media {
            if external_media.validated_reference().is_err() {
                counts.missing += 1;
                continue;
            }

            match external_media.mime_type.as_str() {
                "image/gif" => counts.gifs += 1,
                value if value.starts_with("image/") => counts.photos += 1,
                value if value.starts_with("video/") => counts.videos += 1,
                _ => counts.photos += 1,
            }
        }

        Ok(counts)
    }

    fn insert_validation_errors(
        transaction: &Transaction<'_>,
        post_id: i64,
        report: &PostValidationReport,
    ) -> Result<(), DbError> {
        let mut by_account: BTreeMap<i64, Vec<String>> = BTreeMap::new();

        for error in &report.errors {
            if error.account_id > 0 {
                by_account
                    .entry(error.account_id)
                    .or_default()
                    .push(error.message.clone());
            }
        }

        for (account_id, messages) in by_account {
            transaction.execute(
                "UPDATE post_accounts
                 SET errors_json = ?3
                 WHERE post_id = ?1 AND account_id = ?2",
                params![post_id, account_id, serde_json::to_string(&messages)?],
            )?;
        }

        Ok(())
    }

    fn calendar_window(
        &self,
        connection: &Connection,
        request: &ValidatedPostQuery,
    ) -> Result<Option<PostCalendarWindow>, DbError> {
        let Some(calendar_type) = request.calendar_type.as_ref() else {
            return Ok(None);
        };
        let Some(selected_date) = request.date.as_ref() else {
            return Ok(None);
        };

        let selected = NaiveDate::parse_from_str(selected_date, "%Y-%m-%d")
            .map_err(|_| DbError::Validation("date must use YYYY-MM-DD format".to_string()))?;
        let (start, end) = match calendar_type.as_str() {
            "month" => {
                let start = selected
                    .with_day(1)
                    .expect("day 1 is valid for every month")
                    - Duration::days(10);
                let next_month = if selected.month() == 12 {
                    NaiveDate::from_ymd_opt(selected.year() + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(selected.year(), selected.month() + 1, 1)
                }
                .expect("next month should be valid");
                let end = next_month - Duration::days(1) + Duration::days(10);

                (start, end)
            }
            "week" => {
                let week_starts_on = self
                    .setting_value::<u8>(connection, "week_starts_on")?
                    .unwrap_or(AppSettings::default().week_starts_on);
                let offset = if week_starts_on == 0 {
                    selected.weekday().num_days_from_sunday()
                } else {
                    selected.weekday().num_days_from_monday()
                };
                let start = selected - Duration::days(i64::from(offset));

                (start, start + Duration::days(6))
            }
            "day" => (selected, selected),
            _ => return Err(DbError::Validation("unsupported calendar_type".to_string())),
        };

        Ok(Some(PostCalendarWindow {
            calendar_type: calendar_type.clone(),
            selected_date: selected.to_string(),
            start_date: start.to_string(),
            end_date: end.to_string(),
        }))
    }

    fn post_query_filter(
        &self,
        request: &ValidatedPostQuery,
        calendar_window: Option<&PostCalendarWindow>,
    ) -> Result<(String, Vec<SqlValue>), DbError> {
        let mut clauses = vec!["p.deleted_at IS NULL".to_string()];
        let mut values = Vec::new();

        if let Some(status) = request.status.as_ref() {
            clauses.push("p.status = ?".to_string());
            values.push(SqlValue::Integer(post_status_filter_code(status)?));
        }

        if let Some(status) = request.exclude_status.as_ref() {
            clauses.push("p.status != ?".to_string());
            values.push(SqlValue::Integer(post_status_filter_code(status)?));
        }

        if let Some(keyword) = request.keyword.as_ref() {
            clauses.push(
                "EXISTS (
                    SELECT 1
                    FROM post_versions pv_filter
                    WHERE pv_filter.post_id = p.id
                      AND LOWER(COALESCE(pv_filter.content_json, '')) LIKE ?
                )"
                .to_string(),
            );
            values.push(SqlValue::Text(format!(
                "%{}%",
                keyword.to_ascii_lowercase()
            )));
        }

        if !request.accounts.is_empty() {
            clauses.push(format!(
                "EXISTS (
                    SELECT 1
                    FROM post_accounts pa_filter
                    WHERE pa_filter.post_id = p.id
                      AND pa_filter.account_id IN ({})
                )",
                placeholders(request.accounts.len())
            ));
            values.extend(request.accounts.iter().map(|id| SqlValue::Integer(*id)));
        }

        if !request.tags.is_empty() {
            clauses.push(format!(
                "EXISTS (
                    SELECT 1
                    FROM tag_post tp_filter
                    WHERE tp_filter.post_id = p.id
                      AND tp_filter.tag_id IN ({})
                )",
                placeholders(request.tags.len())
            ));
            values.extend(request.tags.iter().map(|id| SqlValue::Integer(*id)));
        }

        if let Some(window) = calendar_window {
            clauses.push(
                "p.scheduled_at IS NOT NULL
                 AND substr(p.scheduled_at, 1, 10) >= ?
                 AND substr(p.scheduled_at, 1, 10) <= ?"
                    .to_string(),
            );
            values.push(SqlValue::Text(window.start_date.clone()));
            values.push(SqlValue::Text(window.end_date.clone()));
        }

        Ok((clauses.join(" AND "), values))
    }

    fn count_matching_posts(
        &self,
        connection: &Connection,
        where_sql: &str,
        values: &[SqlValue],
    ) -> Result<i64, DbError> {
        let query = format!("SELECT COUNT(*) FROM posts p WHERE {where_sql}");

        Ok(connection.query_row(&query, params_from_iter(values.iter()), |row| row.get(0))?)
    }

    fn filtered_posts(
        &self,
        connection: &Connection,
        where_sql: &str,
        values: &[SqlValue],
        request: &ValidatedPostQuery,
    ) -> Result<Vec<PostSummary>, DbError> {
        let order_by = if request.calendar_type.is_some() {
            "ORDER BY p.scheduled_at ASC, p.id ASC"
        } else {
            "ORDER BY COALESCE(p.published_at, p.scheduled_at, p.updated_at) DESC, p.id DESC"
        };
        let query = format!(
            "SELECT p.id, p.uuid, p.status, p.schedule_status, p.scheduled_at, p.published_at,
                    COUNT(DISTINCT pa.account_id) AS account_count,
                    COUNT(DISTINCT tp.tag_id) AS tag_count,
                    (
                        SELECT pv.content_json
                        FROM post_versions pv
                        WHERE pv.post_id = p.id
                        ORDER BY pv.is_original DESC, pv.id ASC
                        LIMIT 1
                    ) AS content_json,
                    p.created_at, p.updated_at
             FROM posts p
             LEFT JOIN post_accounts pa ON pa.post_id = p.id
             LEFT JOIN tag_post tp ON tp.post_id = p.id
             WHERE {where_sql}
             GROUP BY p.id
             {order_by}
             LIMIT ?
             OFFSET ?"
        );
        let mut values = values.to_vec();
        values.push(SqlValue::Integer(request.limit));
        values.push(SqlValue::Integer((request.page - 1) * request.limit));
        let mut statement = connection.prepare(&query)?;
        let rows = statement.query_map(params_from_iter(values.iter()), map_post_summary)?;
        let mut posts = collect_rows(rows)?;
        self.hydrate_post_summaries(connection, &mut posts)?;

        Ok(posts)
    }

    fn dashboard_posts(
        &self,
        connection: &Connection,
        where_sql: &str,
        order_by: &str,
        limit: i64,
    ) -> Result<Vec<PostSummary>, DbError> {
        let query = format!(
            "SELECT p.id, p.uuid, p.status, p.schedule_status, p.scheduled_at, p.published_at,
                    COUNT(DISTINCT pa.account_id) AS account_count,
                    COUNT(DISTINCT tp.tag_id) AS tag_count,
                    (
                        SELECT pv.content_json
                        FROM post_versions pv
                        WHERE pv.post_id = p.id
                        ORDER BY pv.is_original DESC, pv.id ASC
                        LIMIT 1
                    ) AS content_json,
                    p.created_at, p.updated_at
             FROM posts p
             LEFT JOIN post_accounts pa ON pa.post_id = p.id
             LEFT JOIN tag_post tp ON tp.post_id = p.id
             WHERE {where_sql}
             GROUP BY p.id
             {order_by}
             LIMIT ?1"
        );
        let mut statement = connection.prepare(&query)?;
        let rows = statement.query_map(params![limit], map_post_summary)?;
        let mut posts = collect_rows(rows)?;
        self.hydrate_post_summaries(connection, &mut posts)?;

        Ok(posts)
    }

    fn dashboard_provider_summaries(
        &self,
        connection: &Connection,
    ) -> Result<Vec<DashboardProviderSummary>, DbError> {
        let mut statement = connection.prepare(
            "SELECT provider,
                    COUNT(*) AS accounts,
                    SUM(CASE WHEN authorized = 1 THEN 1 ELSE 0 END) AS authorized_accounts
             FROM accounts
             GROUP BY provider
             ORDER BY provider ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(DashboardProviderSummary {
                provider: row.get(0)?,
                accounts: row.get(1)?,
                authorized_accounts: row.get(2)?,
            })
        })?;

        collect_rows(rows)
    }

    fn has_failed_posts(&self, connection: &Connection) -> Result<bool, DbError> {
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL AND status = 3",
            [],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    fn list_accounts(
        &self,
        connection: &Connection,
        limit: i64,
    ) -> Result<Vec<AccountSummary>, DbError> {
        let mut statement = connection.prepare(
            "SELECT id, uuid, name, username, provider, provider_id, authorized, avatar_path,
                    access_token_secret_ref, created_at, updated_at
             FROM accounts
             ORDER BY updated_at DESC, id DESC
             LIMIT ?1",
        )?;

        let rows = statement.query_map(params![limit], |row| {
            Ok(AccountSummary {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                username: row.get(3)?,
                provider: row.get(4)?,
                provider_id: row.get(5)?,
                authorized: row.get::<_, i64>(6)? == 1,
                avatar_path: row.get(7)?,
                access_token_secret_ref: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        collect_rows(rows)
    }

    fn account_by_provider_id(
        &self,
        connection: &Connection,
        provider: &str,
        provider_id: &str,
    ) -> Result<AccountSummary, DbError> {
        Ok(connection.query_row(
            "SELECT id, uuid, name, username, provider, provider_id, authorized, avatar_path,
                    access_token_secret_ref, created_at, updated_at
             FROM accounts
             WHERE provider = ?1 AND provider_id = ?2",
            params![provider, provider_id],
            |row| {
                Ok(AccountSummary {
                    id: row.get(0)?,
                    uuid: row.get(1)?,
                    name: row.get(2)?,
                    username: row.get(3)?,
                    provider: row.get(4)?,
                    provider_id: row.get(5)?,
                    authorized: row.get::<_, i64>(6)? == 1,
                    avatar_path: row.get(7)?,
                    access_token_secret_ref: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )?)
    }

    fn import_job_account(&self, account_id: i64) -> Result<(String, String), DbError> {
        let connection = self.connection()?;

        connection
            .query_row(
                "SELECT uuid, provider
                 FROM accounts
                 WHERE id = ?1",
                params![account_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| DbError::Validation(format!("account {account_id} not found")))
    }

    fn set_account_authorized(&self, uuid: &str, authorized: bool) -> Result<(), DbError> {
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE accounts
             SET authorized = ?2,
                 updated_at = CURRENT_TIMESTAMP
             WHERE uuid = ?1",
            params![uuid, if authorized { 1 } else { 0 }],
        )?;

        if changed == 0 {
            return Err(DbError::Validation("account not found".to_string()));
        }

        Ok(())
    }

    fn list_services(
        &self,
        connection: &Connection,
        limit: i64,
    ) -> Result<Vec<ServiceSummary>, DbError> {
        let mut statement = connection.prepare(
            "SELECT id, name, configuration_secret_ref, configuration_json, active
             FROM services
             ORDER BY name ASC, id ASC
             LIMIT ?1",
        )?;

        let rows = statement.query_map(params![limit], map_service_summary)?;

        collect_rows(rows)
    }

    fn service_by_name(
        &self,
        connection: &Connection,
        name: &str,
    ) -> Result<ServiceSummary, DbError> {
        Ok(connection.query_row(
            "SELECT id, name, configuration_secret_ref, configuration_json, active
             FROM services
             WHERE name = ?1",
            params![name],
            map_service_summary,
        )?)
    }

    fn list_posts(&self, connection: &Connection, limit: i64) -> Result<Vec<PostSummary>, DbError> {
        let mut statement = connection.prepare(
            "SELECT p.id, p.uuid, p.status, p.schedule_status, p.scheduled_at, p.published_at,
                    COUNT(DISTINCT pa.account_id) AS account_count,
                    COUNT(DISTINCT tp.tag_id) AS tag_count,
                    (
                        SELECT pv.content_json
                        FROM post_versions pv
                        WHERE pv.post_id = p.id
                        ORDER BY pv.is_original DESC, pv.id ASC
                        LIMIT 1
                    ) AS content_json,
                    p.created_at, p.updated_at
             FROM posts p
             LEFT JOIN post_accounts pa ON pa.post_id = p.id
             LEFT JOIN tag_post tp ON tp.post_id = p.id
             WHERE p.deleted_at IS NULL
             GROUP BY p.id
             ORDER BY COALESCE(p.scheduled_at, p.published_at, p.updated_at) DESC, p.id DESC
             LIMIT ?1",
        )?;

        let rows = statement.query_map(params![limit], map_post_summary)?;
        let mut posts = collect_rows(rows)?;
        self.hydrate_post_summaries(connection, &mut posts)?;

        Ok(posts)
    }

    fn post_by_id(&self, connection: &Connection, id: i64) -> Result<PostSummary, DbError> {
        let mut post = connection.query_row(
            "SELECT p.id, p.uuid, p.status, p.schedule_status, p.scheduled_at, p.published_at,
                    COUNT(DISTINCT pa.account_id) AS account_count,
                    COUNT(DISTINCT tp.tag_id) AS tag_count,
                    (
                        SELECT pv.content_json
                        FROM post_versions pv
                        WHERE pv.post_id = p.id
                        ORDER BY pv.is_original DESC, pv.id ASC
                        LIMIT 1
                    ) AS content_json,
                    p.created_at, p.updated_at
             FROM posts p
             LEFT JOIN post_accounts pa ON pa.post_id = p.id
             LEFT JOIN tag_post tp ON tp.post_id = p.id
             WHERE p.id = ?1 AND p.deleted_at IS NULL
             GROUP BY p.id",
            params![id],
            map_post_summary,
        )?;
        self.hydrate_post_summaries(connection, std::slice::from_mut(&mut post))?;

        Ok(post)
    }

    fn post_detail_by_uuid(
        &self,
        connection: &Connection,
        uuid: &str,
    ) -> Result<PostDetail, DbError> {
        let (id, uuid, status, schedule_status, scheduled_at, published_at): (
            i64,
            String,
            i64,
            i64,
            Option<String>,
            Option<String>,
        ) = connection
            .query_row(
                "SELECT id, uuid, status, schedule_status, scheduled_at, published_at
                 FROM posts
                 WHERE uuid = ?1 AND deleted_at IS NULL",
                params![uuid],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| DbError::Validation("post not found".to_string()))?;

        Ok(PostDetail {
            id,
            uuid,
            status: post_status_label(status, schedule_status).to_string(),
            schedule_status: post_schedule_status_label(schedule_status).to_string(),
            scheduled_at,
            published_at,
            accounts: self.post_account_ids(connection, id)?,
            tags: self.post_tag_ids(connection, id)?,
            versions: self.post_versions(connection, id)?,
        })
    }

    fn post_account_ids(&self, connection: &Connection, post_id: i64) -> Result<Vec<i64>, DbError> {
        let mut statement = connection.prepare(
            "SELECT account_id
             FROM post_accounts
             WHERE post_id = ?1
             ORDER BY id ASC",
        )?;
        let rows = statement.query_map(params![post_id], |row| row.get::<_, i64>(0))?;

        collect_rows(rows)
    }

    fn post_tag_ids(&self, connection: &Connection, post_id: i64) -> Result<Vec<i64>, DbError> {
        let mut statement = connection.prepare(
            "SELECT tag_id
             FROM tag_post
             WHERE post_id = ?1
             ORDER BY id ASC",
        )?;
        let rows = statement.query_map(params![post_id], |row| row.get::<_, i64>(0))?;

        collect_rows(rows)
    }

    fn hydrate_post_summaries(
        &self,
        connection: &Connection,
        posts: &mut [PostSummary],
    ) -> Result<(), DbError> {
        for post in posts {
            post.accounts = self.post_list_accounts(connection, post.id)?;
            post.tags = self.post_list_tags(connection, post.id)?;
            post.media = self.media_library_items_by_ids(connection, &post.media_ids)?;
            post.failure_errors = self.post_failure_errors(connection, post.id)?;
        }

        Ok(())
    }

    fn post_failure_errors(
        &self,
        connection: &Connection,
        post_id: i64,
    ) -> Result<Vec<String>, DbError> {
        let mut statement = connection.prepare(
            "SELECT errors_json
             FROM post_accounts
             WHERE post_id = ?1 AND errors_json IS NOT NULL
             ORDER BY id ASC",
        )?;
        let rows = statement.query_map(params![post_id], |row| row.get::<_, String>(0))?;
        let mut messages = Vec::new();
        let mut seen = BTreeSet::new();

        for row in rows {
            for message in post_account_error_messages(&row?) {
                if seen.insert(message.clone()) {
                    messages.push(message);
                }

                if messages.len() >= 6 {
                    return Ok(messages);
                }
            }
        }

        Ok(messages)
    }

    fn post_list_accounts(
        &self,
        connection: &Connection,
        post_id: i64,
    ) -> Result<Vec<PostListAccount>, DbError> {
        let mut statement = connection.prepare(
            "SELECT a.id, a.uuid, a.name, a.username, a.provider, a.avatar_path, a.authorized
             FROM accounts a
             INNER JOIN post_accounts pa ON pa.account_id = a.id
             WHERE pa.post_id = ?1
             ORDER BY pa.id ASC",
        )?;
        let rows = statement.query_map(params![post_id], |row| {
            Ok(PostListAccount {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                username: row.get(3)?,
                provider: row.get(4)?,
                avatar_path: row.get(5)?,
                authorized: row.get::<_, i64>(6)? == 1,
            })
        })?;

        collect_rows(rows)
    }

    fn post_list_tags(
        &self,
        connection: &Connection,
        post_id: i64,
    ) -> Result<Vec<PostListTag>, DbError> {
        let mut statement = connection.prepare(
            "SELECT t.id, t.uuid, t.name, t.hex_color
             FROM tags t
             INNER JOIN tag_post tp ON tp.tag_id = t.id
             WHERE tp.post_id = ?1
             ORDER BY tp.id ASC",
        )?;
        let rows = statement.query_map(params![post_id], |row| {
            Ok(PostListTag {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                hex_color: row.get(3)?,
            })
        })?;

        collect_rows(rows)
    }

    fn media_library_items_by_ids(
        &self,
        connection: &Connection,
        media_ids: &[i64],
    ) -> Result<Vec<MediaLibraryItem>, DbError> {
        let mut items = Vec::new();

        for media_id in media_ids {
            if let Some(item) = self.media_library_item_by_id(connection, *media_id)? {
                items.push(item);
            }
        }

        Ok(items)
    }

    fn media_library_item_by_id(
        &self,
        connection: &Connection,
        media_id: i64,
    ) -> Result<Option<MediaLibraryItem>, DbError> {
        Ok(connection
            .query_row(
                "SELECT id, uuid, name, mime_type, disk, path, size, size_total, conversions_json
                 FROM media
                 WHERE id = ?1",
                params![media_id],
                |row| {
                    let id = row.get::<_, i64>(0)?;
                    let uuid = row.get::<_, String>(1)?;
                    let name = row.get::<_, String>(2)?;
                    let mime_type = row.get::<_, String>(3)?;
                    let disk = row.get::<_, String>(4)?;
                    let path = row.get::<_, String>(5)?;
                    let size = row.get::<_, i64>(6)?;
                    let size_total = row.get::<_, i64>(7)?;
                    let conversions_json = row.get::<_, Option<String>>(8)?;
                    let media_type = media_resource_type(&mime_type);
                    let is_video = media_type == "video";
                    let url = self.media_resource_path(&disk, &path);
                    let thumb_url = if media_type == "gif" {
                        url.clone()
                    } else {
                        self.media_conversion_resource_path(conversions_json.as_deref(), "thumb")
                    };

                    Ok(MediaLibraryItem {
                        id,
                        uuid,
                        name,
                        mime_type,
                        media_type,
                        url,
                        thumb_url,
                        is_video,
                        disk,
                        path,
                        size,
                        size_total,
                        conversion_count: media_conversion_count(conversions_json),
                    })
                },
            )
            .optional()?)
    }

    fn post_versions(
        &self,
        connection: &Connection,
        post_id: i64,
    ) -> Result<Vec<PostVersionForm>, DbError> {
        let mut statement = connection.prepare(
            "SELECT account_id, is_original, content_json
             FROM post_versions
             WHERE post_id = ?1
             ORDER BY id ASC",
        )?;
        let rows = statement.query_map(params![post_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;

        collect_rows(rows)?
            .into_iter()
            .map(|(account_id, is_original, content_json)| {
                let content = content_json
                    .as_deref()
                    .map(serde_json::from_str::<Vec<PostContentBlock>>)
                    .transpose()?
                    .unwrap_or_default();

                Ok(PostVersionForm {
                    account_id,
                    is_original: is_original == 1,
                    content,
                })
            })
            .collect()
    }

    fn upsert_pending_publish_job(
        transaction: &Transaction<'_>,
        post_id: i64,
        run_at: &str,
    ) -> Result<(), DbError> {
        let payload = publish_job_payload(post_id);
        let idempotency_key = publish_job_idempotency_key(post_id);
        let changed = transaction.execute(
            "UPDATE job_queue
             SET run_at = ?2,
                 idempotency_key = ?3,
                 failed_at = NULL,
                 completed_at = NULL,
                 last_error = NULL,
                 updated_at = CURRENT_TIMESTAMP
             WHERE kind = 'publish_post'
               AND status = 'pending'
               AND (
                    idempotency_key = ?3
                    OR (idempotency_key IS NULL AND payload_json = ?1)
               )",
            params![payload, run_at, idempotency_key],
        )?;

        if changed == 0 {
            transaction.execute(
                "INSERT INTO job_queue (
                    kind, payload_json, idempotency_key, status, attempts, run_at, created_at, updated_at
                 ) VALUES (
                    'publish_post', ?1, ?2, 'pending', 0, ?3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
                 )",
                params![payload, idempotency_key, run_at],
            )?;
        }

        Ok(())
    }

    fn cancel_pending_publish_jobs(
        transaction: &Transaction<'_>,
        post_id: i64,
        reason: &str,
    ) -> Result<(), DbError> {
        transaction.execute(
            "UPDATE job_queue
             SET status = 'cancelled',
                 last_error = ?3,
                 updated_at = CURRENT_TIMESTAMP
             WHERE kind = 'publish_post'
               AND status = 'pending'
               AND (
                    idempotency_key = ?1
                    OR (idempotency_key IS NULL AND payload_json = ?2)
               )",
            params![
                publish_job_idempotency_key(post_id),
                publish_job_payload(post_id),
                reason
            ],
        )?;

        Ok(())
    }

    fn list_media(
        &self,
        connection: &Connection,
        limit: i64,
    ) -> Result<Vec<MediaSummary>, DbError> {
        let mut statement = connection.prepare(
            "SELECT id, uuid, name, mime_type, disk, path, size, size_total, conversions_json,
                    created_at, updated_at
             FROM media
             ORDER BY created_at DESC, id DESC
             LIMIT ?1",
        )?;

        let rows = statement.query_map(params![limit], map_media_summary)?;

        collect_rows(rows)
    }

    fn list_media_library(
        &self,
        connection: &Connection,
        request: &ValidatedMediaLibraryQuery,
    ) -> Result<Vec<MediaLibraryItem>, DbError> {
        let mut clauses = Vec::new();
        let mut values = Vec::new();

        if let Some(keyword) = request.keyword.as_deref() {
            let pattern = format!("%{keyword}%");
            clauses.push(
                "(LOWER(name) LIKE ? OR LOWER(path) LIKE ? OR LOWER(mime_type) LIKE ?)".to_string(),
            );
            values.push(SqlValue::Text(pattern.clone()));
            values.push(SqlValue::Text(pattern.clone()));
            values.push(SqlValue::Text(pattern));
        }

        if let Some(media_type) = request.media_type.as_deref() {
            clauses.push(match media_type {
                "image" => "LOWER(mime_type) LIKE 'image/%' AND LOWER(mime_type) <> 'image/gif'"
                    .to_string(),
                "gif" => "LOWER(mime_type) = 'image/gif'".to_string(),
                "video" => "LOWER(mime_type) LIKE 'video/%'".to_string(),
                "file" => {
                    "LOWER(mime_type) NOT LIKE 'image/%' AND LOWER(mime_type) NOT LIKE 'video/%'"
                        .to_string()
                }
                _ => unreachable!("media_type is validated before querying"),
            });
        }

        let where_sql = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };
        let query = format!(
            "SELECT id, uuid, name, mime_type, disk, path, size, size_total, conversions_json
             FROM media
             {where_sql}
             ORDER BY created_at DESC, id DESC
             LIMIT ?"
        );
        values.push(SqlValue::Integer(request.limit));

        let mut statement = connection.prepare(&query)?;
        let rows = statement.query_map(params_from_iter(values.iter()), |row| {
            let id = row.get::<_, i64>(0)?;
            let uuid = row.get::<_, String>(1)?;
            let name = row.get::<_, String>(2)?;
            let mime_type = row.get::<_, String>(3)?;
            let disk = row.get::<_, String>(4)?;
            let path = row.get::<_, String>(5)?;
            let size = row.get::<_, i64>(6)?;
            let size_total = row.get::<_, i64>(7)?;
            let conversions_json = row.get::<_, Option<String>>(8)?;
            let media_type = media_resource_type(&mime_type);
            let is_video = media_type == "video";
            let url = self.media_resource_path(&disk, &path);
            let thumb_url = if media_type == "gif" {
                url.clone()
            } else {
                self.media_conversion_resource_path(conversions_json.as_deref(), "thumb")
            };

            Ok(MediaLibraryItem {
                id,
                uuid,
                name,
                mime_type,
                media_type,
                url,
                thumb_url,
                is_video,
                disk,
                path,
                size,
                size_total,
                conversion_count: media_conversion_count(conversions_json),
            })
        })?;

        collect_rows(rows)
    }

    fn media_by_uuid(&self, connection: &Connection, uuid: &str) -> Result<MediaSummary, DbError> {
        Ok(connection.query_row(
            "SELECT id, uuid, name, mime_type, disk, path, size, size_total, conversions_json,
                    created_at, updated_at
             FROM media
             WHERE uuid = ?1",
            params![uuid],
            map_media_summary,
        )?)
    }

    fn managed_media_file_paths_by_uuid(
        &self,
        connection: &Connection,
        uuid: &str,
    ) -> Result<Option<Vec<PathBuf>>, DbError> {
        let row = connection
            .query_row(
                "SELECT disk, path, conversions_json
                 FROM media
                 WHERE uuid = ?1",
                params![uuid],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?;

        Ok(row.map(|(disk, path, conversions_json)| {
            self.managed_media_file_paths(&disk, &path, conversions_json.as_deref())
        }))
    }

    fn list_tags(&self, connection: &Connection, limit: i64) -> Result<Vec<TagSummary>, DbError> {
        let mut statement = connection.prepare(
            "SELECT t.id, t.uuid, t.name, t.hex_color, COUNT(tp.post_id) AS post_count,
                    t.created_at, t.updated_at
             FROM tags t
             LEFT JOIN tag_post tp ON tp.tag_id = t.id
             GROUP BY t.id
             ORDER BY t.name ASC, t.id ASC
             LIMIT ?1",
        )?;

        let rows = statement.query_map(params![limit], |row| {
            Ok(TagSummary {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                hex_color: row.get(3)?,
                post_count: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        collect_rows(rows)
    }

    fn tag_by_uuid(&self, connection: &Connection, uuid: &str) -> Result<TagSummary, DbError> {
        Ok(connection.query_row(
            "SELECT t.id, t.uuid, t.name, t.hex_color, COUNT(tp.post_id) AS post_count,
                    t.created_at, t.updated_at
             FROM tags t
             LEFT JOIN tag_post tp ON tp.tag_id = t.id
             WHERE t.uuid = ?1
             GROUP BY t.id",
            params![uuid],
            |row| {
                Ok(TagSummary {
                    id: row.get(0)?,
                    uuid: row.get(1)?,
                    name: row.get(2)?,
                    hex_color: row.get(3)?,
                    post_count: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )?)
    }

    fn list_jobs(&self, connection: &Connection, limit: i64) -> Result<Vec<JobSummary>, DbError> {
        let mut statement = connection.prepare(
            "SELECT id, kind, status, attempts, run_at, last_error, idempotency_key
             FROM job_queue
             ORDER BY run_at ASC, id ASC
             LIMIT ?1",
        )?;

        let rows = statement.query_map(params![limit], map_job_summary)?;

        collect_rows(rows)
    }

    fn list_rate_limits(
        &self,
        connection: &Connection,
        limit: i64,
    ) -> Result<Vec<RateLimitSummary>, DbError> {
        let mut statement = connection.prepare(
            "SELECT id, scope, retry_after_at, payload_json, created_at, updated_at
             FROM rate_limits
             ORDER BY retry_after_at ASC, id ASC
             LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit], map_rate_limit_summary)?;

        collect_rows(rows)
    }

    #[cfg(test)]
    fn rate_limit_by_scope(
        &self,
        connection: &Connection,
        scope: &str,
    ) -> Result<RateLimitSummary, DbError> {
        Ok(connection.query_row(
            "SELECT id, scope, retry_after_at, payload_json, created_at, updated_at
             FROM rate_limits
             WHERE scope = ?1",
            params![scope],
            map_rate_limit_summary,
        )?)
    }

    fn job_by_id(&self, connection: &Connection, id: i64) -> Result<JobSummary, DbError> {
        Ok(connection.query_row(
            "SELECT id, kind, status, attempts, run_at, last_error, idempotency_key
             FROM job_queue
             WHERE id = ?1",
            params![id],
            map_job_summary,
        )?)
    }

    fn jobs_by_ids(
        &self,
        connection: &Connection,
        ids: &[i64],
    ) -> Result<Vec<JobSummary>, DbError> {
        ids.iter()
            .map(|id| self.job_by_id(connection, *id))
            .collect::<Result<Vec<_>, _>>()
    }

    fn connection(&self) -> Result<Connection, DbError> {
        let connection = Connection::open(&self.path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;

        Ok(connection)
    }

    fn migrate(&self) -> Result<(), DbError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;

        transaction.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                filename TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;

        let mut applied_versions: BTreeSet<u16> = {
            let mut statement = transaction.prepare("SELECT version FROM schema_migrations")?;
            statement
                .query_map([], |row| row.get(0))?
                .collect::<Result<_, _>>()?
        };

        for migration in SCHEMA_MIGRATIONS {
            if applied_versions.contains(&migration.version) {
                continue;
            }

            transaction.execute_batch(migration.sql)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, filename) VALUES (?1, ?2)",
                params![migration.version, migration.filename],
            )?;
            applied_versions.insert(migration.version);
        }

        transaction.commit()?;

        Ok(())
    }

    fn setting_value<T>(&self, connection: &Connection, name: &str) -> Result<Option<T>, DbError>
    where
        T: serde::de::DeserializeOwned,
    {
        let payload: Option<String> = connection
            .query_row(
                "SELECT payload_json FROM settings WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()?;

        payload
            .map(|value| serde_json::from_str(&value).map_err(DbError::Json))
            .transpose()
    }

    fn save_setting_value<T>(
        &self,
        connection: &Connection,
        name: &str,
        value: &T,
    ) -> Result<(), DbError>
    where
        T: Serialize,
    {
        let payload = serde_json::to_string(value)?;

        connection.execute(
            "INSERT INTO settings (name, payload_json)
             VALUES (?1, ?2)
             ON CONFLICT(name) DO UPDATE SET payload_json = excluded.payload_json",
            params![name, payload],
        )?;

        Ok(())
    }
}

fn count_rows(connection: &Connection, table_name: &str) -> Result<i64, DbError> {
    let query = format!("SELECT COUNT(*) FROM {table_name}");

    Ok(connection.query_row(&query, [], |row| row.get(0))?)
}

fn count_matching_rows(
    connection: &Connection,
    table_name: &str,
    where_sql: &str,
) -> Result<i64, DbError> {
    let query = format!("SELECT COUNT(*) FROM {table_name} WHERE {where_sql}");

    Ok(connection.query_row(&query, [], |row| row.get(0))?)
}

fn validate_backup_database(path: &Path) -> Result<(), DbError> {
    let connection = Connection::open(path)?;

    for table in [
        "schema_migrations",
        "settings",
        "accounts",
        "posts",
        "media",
        "job_queue",
    ] {
        let exists: i64 = connection.query_row(
            "SELECT COUNT(*)
             FROM sqlite_master
             WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )?;

        if exists == 0 {
            return Err(DbError::Validation(format!(
                "backup database is missing required table {table}",
            )));
        }
    }

    Ok(())
}

fn validate_backup_manifest(path: &Path) -> Result<(), DbError> {
    let metadata = fs::metadata(path)?;

    if metadata.len() > 1024 * 1024 {
        return Err(DbError::Validation(
            "backup manifest is too large".to_string(),
        ));
    }

    let manifest = serde_json::from_slice::<serde_json::Value>(&fs::read(path)?)?;
    let product = manifest
        .get("product")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    if product != "Dust Wave Social" {
        return Err(DbError::Validation(
            "backup manifest product is not Dust Wave Social".to_string(),
        ));
    }

    let generated_at = manifest
        .get("generated_at")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    if generated_at.trim().is_empty() {
        return Err(DbError::Validation(
            "backup manifest generated_at is required".to_string(),
        ));
    }

    let schema_version = manifest
        .get("schema_version")
        .and_then(json_number_to_i64)
        .ok_or_else(|| DbError::Validation("backup schema_version is required".to_string()))?;

    if schema_version < 1 {
        return Err(DbError::Validation(
            "backup schema_version must be positive".to_string(),
        ));
    }

    if schema_version > i64::from(CURRENT_SCHEMA_VERSION) {
        return Err(DbError::Validation(format!(
            "backup schema version {schema_version} is newer than this app supports ({CURRENT_SCHEMA_VERSION})",
        )));
    }

    if manifest.get("database").and_then(serde_json::Value::as_str)
        != Some("dust-wave-social.sqlite3")
    {
        return Err(DbError::Validation(
            "backup manifest database must be dust-wave-social.sqlite3".to_string(),
        ));
    }

    if manifest.get("media").and_then(serde_json::Value::as_str) != Some("media") {
        return Err(DbError::Validation(
            "backup manifest media folder must be media".to_string(),
        ));
    }

    if manifest
        .pointer("/secrets/included")
        .and_then(serde_json::Value::as_bool)
        != Some(false)
    {
        return Err(DbError::Validation(
            "backup manifest must declare that secrets are excluded".to_string(),
        ));
    }

    Ok(())
}

fn collect_rows<T>(
    rows: impl Iterator<Item = Result<T, rusqlite::Error>>,
) -> Result<Vec<T>, DbError> {
    rows.collect::<Result<Vec<_>, _>>().map_err(DbError::Sqlite)
}

fn json_number_to_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        .or_else(|| value.as_f64().map(|number| number as i64))
}

fn post_status_filter_code(value: &str) -> Result<i64, DbError> {
    match value {
        "draft" => Ok(0),
        "scheduled" => Ok(1),
        "published" => Ok(2),
        "failed" => Ok(3),
        _ => Err(DbError::Validation(
            "status must be draft, scheduled, published, or failed".to_string(),
        )),
    }
}

fn placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(",")
}

fn publish_job_payload(post_id: i64) -> String {
    serde_json::json!({ "post_id": post_id }).to_string()
}

fn publish_job_idempotency_key(post_id: i64) -> String {
    format!("publish_post:{post_id}")
}

fn format_validation_errors(report: &PostValidationReport) -> String {
    report
        .errors
        .iter()
        .map(|error| error.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

fn mastodon_server_from_account_data(data_json: Option<&str>) -> Result<Option<String>, DbError> {
    let Some(data_json) = data_json else {
        return Ok(None);
    };
    let value = serde_json::from_str::<serde_json::Value>(data_json)?;

    Ok(value
        .get("server")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string))
}

fn should_mark_mastodon_unauthorized(error: &MastodonError) -> bool {
    matches!(error, MastodonError::Unauthorized(_))
}

fn should_mark_twitter_unauthorized(error: &TwitterError) -> bool {
    matches!(error, TwitterError::Unauthorized(_))
}

fn should_mark_facebook_unauthorized(error: &FacebookError) -> bool {
    matches!(error, FacebookError::Unauthorized(_))
}

fn supports_account_imports(provider: &str) -> bool {
    matches!(provider, "twitter" | "facebook_page" | "mastodon")
}

fn twitter_oauth2_account_data(data_json: Option<&str>) -> Result<bool, DbError> {
    let Some(data_json) = data_json else {
        return Ok(false);
    };
    let value = serde_json::from_str::<serde_json::Value>(data_json)?;

    Ok(value.get("auth").and_then(serde_json::Value::as_str) == Some("oauth2_pkce"))
}

fn facebook_page_connected_account_data(data_json: Option<&str>) -> Result<bool, DbError> {
    let Some(data_json) = data_json else {
        return Ok(false);
    };
    let value = serde_json::from_str::<serde_json::Value>(data_json)?;

    Ok(value.get("auth").and_then(serde_json::Value::as_str) == Some("facebook_user"))
}

fn failed_publish_result(
    target: &PublishTarget,
    detail: &str,
) -> Result<PublishAccountResult, DbError> {
    let account_detail = format!(
        "account {} ({}) {detail}",
        target.account_id, target.provider
    );

    Ok(PublishAccountResult {
        post_account_id: target.post_account_id,
        provider_post_id: None,
        data_json: None,
        errors_json: Some(serde_json::to_string(&vec![detail.to_string()])?),
        error_detail: Some(account_detail),
        published_remotely: false,
    })
}

fn publish_success_detail(post_id: i64, remote_count: usize) -> String {
    format!("post {post_id} published to {remote_count} provider account(s)")
}

fn import_twitter_post_rows(
    transaction: &Transaction<'_>,
    account_id: i64,
    posts: &[serde_json::Value],
) -> Result<usize, DbError> {
    let mut imported = 0;

    for post in posts {
        let Some(provider_post_id) = post.get("id").and_then(json_value_to_string) else {
            continue;
        };
        let Some(created_at) = twitter_post_date(post) else {
            continue;
        };
        let public_metrics = post
            .get("public_metrics")
            .unwrap_or(&serde_json::Value::Null);
        let content_json = serde_json::json!({
            "text": post.get("text").and_then(serde_json::Value::as_str).unwrap_or("")
        })
        .to_string();
        let metrics_json = serde_json::json!({
            "user_profile_clicks": public_metrics.get("user_profile_clicks").and_then(json_number_to_i64).unwrap_or(0),
            "impressions": public_metrics.get("impression_count").and_then(json_number_to_i64).unwrap_or(0),
            "likes": public_metrics.get("like_count").and_then(json_number_to_i64).unwrap_or(0),
            "replies": public_metrics.get("reply_count").and_then(json_number_to_i64).unwrap_or(0),
            "retweets": public_metrics.get("retweet_count").and_then(json_number_to_i64).unwrap_or(0),
            "quotes": public_metrics.get("quote_count").and_then(json_number_to_i64).unwrap_or(0),
            "bookmarks": public_metrics.get("bookmark_count").and_then(json_number_to_i64).unwrap_or(0),
        })
        .to_string();

        transaction.execute(
            "INSERT INTO imported_posts (
                account_id, provider_post_id, content_json, metrics_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(account_id, provider_post_id) DO UPDATE SET
                content_json = excluded.content_json,
                metrics_json = excluded.metrics_json,
                created_at = excluded.created_at",
            params![
                account_id,
                provider_post_id,
                content_json,
                metrics_json,
                created_at
            ],
        )?;
        imported += 1;
    }

    Ok(imported)
}

fn process_twitter_metric_days(
    transaction: &Transaction<'_>,
    account_id: i64,
) -> Result<usize, DbError> {
    let mut statement = transaction.prepare(
        "SELECT created_at, metrics_json
         FROM imported_posts
         WHERE account_id = ?1",
    )?;
    let rows = statement.query_map(params![account_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut days = BTreeMap::<String, (i64, i64, i64, i64)>::new();

    for (date, metrics_json) in collect_rows(rows)? {
        let value = serde_json::from_str::<serde_json::Value>(&metrics_json)?;
        let entry = days.entry(date).or_insert((0, 0, 0, 0));

        entry.0 += value.get("likes").and_then(json_number_to_i64).unwrap_or(0);
        entry.1 += value
            .get("replies")
            .and_then(json_number_to_i64)
            .unwrap_or(0);
        entry.2 += value
            .get("retweets")
            .and_then(json_number_to_i64)
            .unwrap_or(0);
        entry.3 += value
            .get("impressions")
            .and_then(json_number_to_i64)
            .unwrap_or(0);
    }

    for (date, (likes, replies, retweets, impressions)) in &days {
        let data_json = serde_json::json!({
            "likes": likes,
            "replies": replies,
            "retweets": retweets,
            "impressions": impressions,
        })
        .to_string();

        transaction.execute(
            "INSERT INTO metrics (account_id, data_json, date)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id, date) DO UPDATE SET data_json = excluded.data_json",
            params![account_id, data_json, date],
        )?;
    }

    Ok(days.len())
}

fn import_facebook_insight_rows(
    transaction: &Transaction<'_>,
    account_id: i64,
    insights: &[serde_json::Value],
) -> Result<usize, DbError> {
    let mut imported = 0;

    for insight in insights {
        let Some(insight_type) = insight
            .get("name")
            .and_then(serde_json::Value::as_str)
            .and_then(facebook_insight_type)
        else {
            continue;
        };
        let Some(values) = insight.get("values").and_then(serde_json::Value::as_array) else {
            continue;
        };

        for item in values {
            let Some(date) = facebook_insight_date(item) else {
                continue;
            };
            let value = item.get("value").and_then(json_number_to_i64).unwrap_or(0);

            transaction.execute(
                "INSERT INTO facebook_insights (account_id, type, value, date, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                 ON CONFLICT(account_id, type, date) DO UPDATE SET
                    value = excluded.value,
                    updated_at = CURRENT_TIMESTAMP",
                params![account_id, insight_type, value, date],
            )?;
            imported += 1;
        }
    }

    Ok(imported)
}

fn import_mastodon_status_rows(
    transaction: &Transaction<'_>,
    account_id: i64,
    statuses: &[serde_json::Value],
) -> Result<usize, DbError> {
    let mut imported = 0;

    for status in statuses {
        let Some(provider_post_id) = status.get("id").and_then(json_value_to_string) else {
            continue;
        };
        let Some(created_at) = mastodon_status_date(status) else {
            continue;
        };
        let content_json = serde_json::json!({
            "text": status.get("content").and_then(serde_json::Value::as_str).unwrap_or("")
        })
        .to_string();
        let metrics_json = serde_json::json!({
            "replies": status.get("replies_count").and_then(json_number_to_i64).unwrap_or(0),
            "reblogs": status.get("reblogs_count").and_then(json_number_to_i64).unwrap_or(0),
            "favourites": status.get("favourites_count").and_then(json_number_to_i64).unwrap_or(0),
        })
        .to_string();

        transaction.execute(
            "INSERT INTO imported_posts (
                account_id, provider_post_id, content_json, metrics_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(account_id, provider_post_id) DO UPDATE SET
                content_json = excluded.content_json,
                metrics_json = excluded.metrics_json,
                created_at = excluded.created_at",
            params![
                account_id,
                provider_post_id,
                content_json,
                metrics_json,
                created_at
            ],
        )?;
        imported += 1;
    }

    Ok(imported)
}

fn process_mastodon_metric_days(
    transaction: &Transaction<'_>,
    account_id: i64,
) -> Result<usize, DbError> {
    let mut statement = transaction.prepare(
        "SELECT created_at, metrics_json
         FROM imported_posts
         WHERE account_id = ?1",
    )?;
    let rows = statement.query_map(params![account_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut days = BTreeMap::<String, (i64, i64, i64)>::new();

    for (date, metrics_json) in collect_rows(rows)? {
        let value = serde_json::from_str::<serde_json::Value>(&metrics_json)?;
        let entry = days.entry(date).or_insert((0, 0, 0));

        entry.0 += value
            .get("replies")
            .and_then(json_number_to_i64)
            .unwrap_or(0);
        entry.1 += value
            .get("reblogs")
            .and_then(json_number_to_i64)
            .unwrap_or(0);
        entry.2 += value
            .get("favourites")
            .and_then(json_number_to_i64)
            .unwrap_or(0);
    }

    for (date, (replies, reblogs, favourites)) in &days {
        let data_json = serde_json::json!({
            "replies": replies,
            "reblogs": reblogs,
            "favourites": favourites,
        })
        .to_string();

        transaction.execute(
            "INSERT INTO metrics (account_id, data_json, date)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id, date) DO UPDATE SET data_json = excluded.data_json",
            params![account_id, data_json, date],
        )?;
    }

    Ok(days.len())
}

fn mastodon_status_date(status: &serde_json::Value) -> Option<String> {
    let created_at = status.get("created_at")?.as_str()?.trim();
    let date = created_at.get(0..10)?;

    (date.len() == 10).then(|| date.to_string())
}

fn twitter_post_date(post: &serde_json::Value) -> Option<String> {
    let created_at = post.get("created_at")?.as_str()?.trim();
    let date = created_at.get(0..10)?;

    (date.len() == 10).then(|| date.to_string())
}

fn facebook_insight_date(item: &serde_json::Value) -> Option<String> {
    let end_time = item.get("end_time")?.as_str()?.trim();
    let date = end_time.get(0..10)?;

    (date.len() == 10).then(|| date.to_string())
}

fn facebook_insight_type(name: &str) -> Option<i64> {
    match name.trim() {
        "page_post_engagements" => Some(2),
        "page_posts_impressions" => Some(3),
        _ => None,
    }
}

fn json_value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => {
            let trimmed = value.trim();

            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_json::Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn media_storage_filename(uuid: &str, source_path: &Path) -> String {
    media_storage_filename_from_extension(
        uuid,
        source_path.extension().and_then(|value| value.to_str()),
    )
}

fn media_storage_filename_from_extension(uuid: &str, extension: Option<&str>) -> String {
    let extension = extension
        .map(|value| {
            value
                .chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|value| !value.is_empty());

    match extension {
        Some(extension) => format!("{uuid}.{extension}"),
        None => uuid.to_string(),
    }
}

fn url_file_name(url: &Url) -> Option<String> {
    url.path_segments()
        .and_then(|segments| {
            segments
                .filter(|segment| !segment.trim().is_empty())
                .next_back()
        })
        .map(str::to_string)
}

fn trim_content_type(value: &str) -> String {
    value
        .split(';')
        .next()
        .unwrap_or(value)
        .trim()
        .to_ascii_lowercase()
}

fn extension_for_mime_type(value: &str) -> Option<&'static str> {
    match value {
        "image/gif" => Some("gif"),
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/png" => Some("png"),
        "video/mp4" => Some("mp4"),
        "video/x-m4v" => Some("m4v"),
        _ => None,
    }
}

fn validate_media_upload(mime_type: &str, size: u64) -> Result<(), DbError> {
    let max_bytes = match mime_type {
        "image/jpg" | "image/jpeg" | "image/png" => MAX_IMAGE_BYTES,
        "image/gif" => MAX_GIF_BYTES,
        "video/mp4" | "video/x-m4v" => MAX_VIDEO_BYTES,
        _ => {
            return Err(DbError::Validation(
                "mime_type must be image/jpg, image/jpeg, image/gif, image/png, video/mp4, or video/x-m4v".to_string(),
            ));
        }
    };

    if size > max_bytes {
        let label = match mime_type {
            "image/gif" => "gif",
            value if value.starts_with("image/") => "image",
            _ => "video",
        };
        let max_mb = max_bytes / 1024 / 1024;

        return Err(DbError::Validation(format!(
            "{label} must not be greater than {max_mb}MB"
        )));
    }

    Ok(())
}

fn media_tool_path(env_var: &str, binary: &str) -> String {
    media_tool_command(env_var, binary)
}

fn ffprobe_duration_seconds(ffprobe: &str, input_path: &Path) -> Option<f64> {
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .ok()
}

fn run_ffmpeg_frame(
    ffmpeg: &str,
    input_path: &Path,
    seconds: f64,
    destination_path: &Path,
) -> Result<bool, DbError> {
    let seconds = format!("{seconds:.3}");
    let output = match Command::new(ffmpeg)
        .args(["-y", "-ss", &seconds, "-i"])
        .arg(input_path)
        .args(["-frames:v", "1", "-q:v", "2"])
        .arg(destination_path)
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(DbError::Io(error)),
    };

    Ok(output.status.success() && destination_path.exists())
}

fn managed_media_file_suffix(path: &str) -> Option<PathBuf> {
    let relative_path = Path::new(path);

    if relative_path.is_absolute() {
        return None;
    }

    let mut components = relative_path.components();

    match components.next() {
        Some(std::path::Component::Normal(prefix)) if prefix.to_str() == Some("media") => {}
        _ => return None,
    }

    let mut suffix = PathBuf::new();
    let mut has_file_name = false;

    for component in components {
        match component {
            std::path::Component::Normal(value) => {
                suffix.push(value);
                has_file_name = true;
            }
            _ => return None,
        }
    }

    has_file_name.then_some(suffix)
}

fn managed_media_conversion_suffixes(conversions_json: Option<&str>) -> Vec<PathBuf> {
    let Some(conversions_json) = conversions_json else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(conversions_json) else {
        return Vec::new();
    };

    let items: Vec<&serde_json::Value> = match &value {
        serde_json::Value::Array(items) => items.iter().collect(),
        serde_json::Value::Object(map) => map.values().collect(),
        _ => Vec::new(),
    };

    items
        .into_iter()
        .filter_map(|item| {
            let disk = item.get("disk").and_then(serde_json::Value::as_str);

            if disk.is_some_and(|value| value != "local") {
                return None;
            }

            item.get("path")
                .and_then(serde_json::Value::as_str)
                .and_then(managed_media_file_suffix)
        })
        .collect()
}

fn media_conversion_entry(
    conversions_json: Option<&str>,
    conversion_name: &str,
) -> Option<serde_json::Value> {
    let conversions_json = conversions_json?;
    let value = serde_json::from_str::<serde_json::Value>(conversions_json).ok()?;

    match value {
        serde_json::Value::Array(items) => items.into_iter().find(|item| {
            item.get("name").and_then(serde_json::Value::as_str) == Some(conversion_name)
        }),
        serde_json::Value::Object(map) => map.get(conversion_name).cloned().or_else(|| {
            map.values()
                .find(|item| {
                    item.get("name").and_then(serde_json::Value::as_str) == Some(conversion_name)
                })
                .cloned()
        }),
        _ => None,
    }
}

fn media_resource_type(mime_type: &str) -> String {
    if mime_type.starts_with("video/") {
        return "video".to_string();
    }

    if mime_type == "image/gif" {
        return "gif".to_string();
    }

    if mime_type.starts_with("image/") {
        return "image".to_string();
    }

    "file".to_string()
}

fn copy_directory_contents(source: &Path, destination: &Path) -> Result<(usize, u64), DbError> {
    fs::create_dir_all(destination)?;

    if !source.exists() {
        return Ok((0, 0));
    }

    let mut file_count = 0;
    let mut byte_count = 0;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            let (nested_files, nested_bytes) =
                copy_directory_contents(&source_path, &destination_path)?;
            file_count += nested_files;
            byte_count += nested_bytes;
        } else if file_type.is_file() {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }

            byte_count += fs::copy(&source_path, &destination_path)?;
            file_count += 1;
        }
    }

    Ok((file_count, byte_count))
}

fn infer_media_mime_type(source_path: &Path) -> &'static str {
    match source_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("gif") => "image/gif",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("m4v") => "video/x-m4v",
        Some("mp4") => "video/mp4",
        _ => "application/octet-stream",
    }
}

fn validation_text(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut inside_tag = false;

    for character in value.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                result.push(' ');
            }
            _ if !inside_tag => result.push(character),
            _ => {}
        }
    }

    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn account_rate_limit_scope(account_id: i64) -> String {
    format!("mixpost-{account_id}-api-limit")
}

fn app_rate_limit_scope(provider: &str) -> String {
    let platform = match provider {
        "facebook_page" | "facebook_group" => "meta",
        other => other,
    };

    format!("mixpost-{platform}-api-limit")
}

fn redact_diagnostics_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(redact_diagnostics_value).collect())
        }
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if is_sensitive_diagnostics_key(&key) {
                        (key, serde_json::Value::String("[redacted]".to_string()))
                    } else {
                        (key, redact_diagnostics_value(value))
                    }
                })
                .collect(),
        ),
        serde_json::Value::String(value) if looks_like_authorization_header(&value) => {
            serde_json::Value::String("[redacted]".to_string())
        }
        other => other,
    }
}

fn is_sensitive_diagnostics_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();

    if key.ends_with("_secret_ref") || key == "configuration_secret_ref" {
        return false;
    }

    key.contains("access_token")
        || key.contains("refresh_token")
        || key.contains("id_token")
        || key.contains("client_secret")
        || key.contains("api_key")
        || key.contains("authorization")
        || key.contains("bearer")
        || key.contains("password")
        || key.contains("credential")
        || key.contains("secret")
}

fn looks_like_authorization_header(value: &str) -> bool {
    let value = value.trim_start().to_ascii_lowercase();

    value.starts_with("bearer ") || value.starts_with("basic ")
}

fn map_service_summary(row: &rusqlite::Row<'_>) -> Result<ServiceSummary, rusqlite::Error> {
    let configuration_json: String = row.get(3)?;

    Ok(ServiceSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        configuration_secret_ref: row.get(2)?,
        configuration: serde_json::from_str(&configuration_json)
            .unwrap_or_else(|_| serde_json::json!({})),
        active: row.get::<_, i64>(4)? == 1,
    })
}

fn map_media_summary(row: &rusqlite::Row<'_>) -> Result<MediaSummary, rusqlite::Error> {
    Ok(MediaSummary {
        id: row.get(0)?,
        uuid: row.get(1)?,
        name: row.get(2)?,
        mime_type: row.get(3)?,
        disk: row.get(4)?,
        path: row.get(5)?,
        size: row.get(6)?,
        size_total: row.get(7)?,
        conversion_count: media_conversion_count(row.get(8)?),
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn map_post_summary(row: &rusqlite::Row<'_>) -> Result<PostSummary, rusqlite::Error> {
    let status: i64 = row.get(2)?;
    let schedule_status: i64 = row.get(3)?;
    let content_json = row.get::<_, Option<String>>(8)?;
    let media_ids = post_media_ids(content_json.as_deref());
    let external_media = post_external_media(content_json.as_deref());

    Ok(PostSummary {
        id: row.get(0)?,
        uuid: row.get(1)?,
        status: post_status_label(status, schedule_status).to_string(),
        schedule_status: post_schedule_status_label(schedule_status).to_string(),
        scheduled_at: row.get(4)?,
        published_at: row.get(5)?,
        account_count: row.get(6)?,
        tag_count: row.get(7)?,
        accounts: Vec::new(),
        tags: Vec::new(),
        media: Vec::new(),
        external_media,
        failure_errors: Vec::new(),
        preview: post_preview(content_json),
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        media_ids,
    })
}

fn post_account_error_messages(errors_json: &str) -> Vec<String> {
    match serde_json::from_str::<serde_json::Value>(errors_json) {
        Ok(serde_json::Value::Array(items)) => items
            .into_iter()
            .filter_map(post_account_error_message)
            .collect(),
        Ok(value) => post_account_error_message(value).into_iter().collect(),
        Err(_) => vec![errors_json.trim().to_string()],
    }
    .into_iter()
    .filter(|message| !message.is_empty())
    .collect()
}

fn post_account_error_message(value: serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.trim().to_string()),
        serde_json::Value::Object(mut map) => map
            .remove("message")
            .and_then(post_account_error_message)
            .or_else(|| map.remove("error").and_then(post_account_error_message)),
        serde_json::Value::Null => None,
        other => Some(other.to_string()),
    }
}

fn post_media_ids(content_json: Option<&str>) -> Vec<i64> {
    let Some(content_json) = content_json else {
        return Vec::new();
    };

    let Ok(blocks) = serde_json::from_str::<Vec<PostContentBlock>>(content_json) else {
        return Vec::new();
    };

    blocks
        .into_iter()
        .flat_map(|block| block.media)
        .filter(|id| *id > 0)
        .collect()
}

fn post_external_media(content_json: Option<&str>) -> Vec<ExternalMediaItem> {
    let Some(content_json) = content_json else {
        return Vec::new();
    };

    let Ok(blocks) = serde_json::from_str::<Vec<PostContentBlock>>(content_json) else {
        return Vec::new();
    };

    blocks
        .into_iter()
        .flat_map(|block| block.external_media)
        .collect()
}

fn map_job_summary(row: &rusqlite::Row<'_>) -> Result<JobSummary, rusqlite::Error> {
    Ok(JobSummary {
        id: row.get(0)?,
        kind: row.get(1)?,
        status: row.get(2)?,
        attempts: row.get(3)?,
        run_at: row.get(4)?,
        last_error: row.get(5)?,
        idempotency_key: row.get(6)?,
    })
}

fn map_rate_limit_summary(row: &rusqlite::Row<'_>) -> Result<RateLimitSummary, rusqlite::Error> {
    let payload_json: Option<String> = row.get(3)?;
    let payload = payload_json
        .as_deref()
        .and_then(|value| serde_json::from_str(value).ok());

    Ok(RateLimitSummary {
        id: row.get(0)?,
        scope: row.get(1)?,
        retry_after_at: row.get(2)?,
        payload,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn normalize_system_log_level(level: &str) -> Result<String, DbError> {
    let level = level.trim().to_ascii_lowercase();

    match level.as_str() {
        "debug" | "info" | "warning" | "error" => Ok(level),
        _ => Err(DbError::Validation(
            "log level must be debug, info, warning, or error".to_string(),
        )),
    }
}

fn truncate_utf8_to_bytes(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }

    let mut end = max_bytes;

    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }

    value[..end].to_string()
}

fn format_system_log_size(bytes: usize) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    if bytes < 1024 * 1024 {
        return format!("{:.1} KB", bytes as f64 / 1024.0);
    }

    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::net::TcpListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn pending_job_count(database: &Database) -> i64 {
        let connection = database.connection().expect("database should open");

        count_matching_rows(&connection, "job_queue", "status = 'pending'")
            .expect("pending jobs should count")
    }

    fn temporary_path(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{prefix}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ))
    }

    fn serve_test_media_response(
        content_type: &'static str,
        body: &'static [u8],
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let address = listener
            .local_addr()
            .expect("test server address should resolve");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test request should arrive");
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );

            stream
                .write_all(header.as_bytes())
                .expect("response header should write");
            stream.write_all(body).expect("response body should write");
        });

        (address.to_string(), handle)
    }

    fn backup_manifest(backup: &LocalBackupExportSummary) -> serde_json::Value {
        serde_json::from_slice(
            &fs::read(&backup.manifest_path).expect("manifest should be readable"),
        )
        .expect("manifest should be json")
    }

    fn write_backup_manifest(backup: &LocalBackupExportSummary, manifest: &serde_json::Value) {
        fs::write(
            &backup.manifest_path,
            serde_json::to_vec_pretty(manifest).expect("manifest should serialize"),
        )
        .expect("manifest should update");
    }

    #[test]
    fn initializes_schema_and_reports_empty_counts() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let connection = database.connection().expect("database should open");

        assert_eq!(count_rows(&connection, "accounts").unwrap(), 0);
        assert_eq!(count_rows(&connection, "posts").unwrap(), 0);
        assert_eq!(count_rows(&connection, "media").unwrap(), 0);
        assert_eq!(
            count_matching_rows(&connection, "job_queue", "status = 'pending'").unwrap(),
            0
        );
        assert_eq!(
            count_rows(&connection, "schema_migrations").unwrap(),
            i64::try_from(SCHEMA_MIGRATIONS.len()).unwrap()
        );

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn initialization_migrations_are_idempotent() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-idempotent-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        Database::initialize_at(&path).expect("database should initialize");
        let database = Database::initialize_at(&path).expect("database should reinitialize");
        let connection = database.connection().expect("database should open");
        let migrations: Vec<(u16, String)> = {
            let mut statement = connection
                .prepare("SELECT version, filename FROM schema_migrations ORDER BY version")
                .expect("migration query should prepare");
            statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .expect("migration query should run")
                .collect::<Result<_, _>>()
                .expect("migration rows should map")
        };
        assert_eq!(
            migrations,
            SCHEMA_MIGRATIONS
                .iter()
                .map(|migration| (migration.version, migration.filename.to_string()))
                .collect::<Vec<_>>()
        );

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn records_exports_and_clears_system_logs() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-logs-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        database
            .record_system_log(
                "INFO",
                "Worker finished",
                Some(serde_json::json!({
                    "access_token": "raw-token",
                    "access_token_secret_ref": "secret://accounts/mastodon/dustwave",
                    "completed": 1,
                    "nested": {
                        "authorization": "Bearer raw-bearer-token",
                        "profile": {
                            "client_secret": "raw-client-secret",
                            "configuration_secret_ref": "secret://services/twitter"
                        }
                    },
                    "headers": ["Basic raw-basic-token"],
                })),
            )
            .expect("system log should record");

        let logs = database.system_logs().expect("system logs should load");

        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].name, SYSTEM_LOG_NAME);
        assert_eq!(logs[0].entry_count, 1);
        assert!(logs[0].contents.contains("[INFO] Worker finished"));
        assert!(logs[0].contents.contains("\"access_token\":\"[redacted]\""));
        assert!(
            logs[0]
                .contents
                .contains("\"authorization\":\"[redacted]\"")
        );
        assert!(
            logs[0]
                .contents
                .contains("\"client_secret\":\"[redacted]\"")
        );
        assert!(logs[0].contents.contains("\"[redacted]\""));
        assert!(
            logs[0]
                .contents
                .contains("\"access_token_secret_ref\":\"secret://accounts/mastodon/dustwave\"")
        );
        assert!(
            logs[0]
                .contents
                .contains("\"configuration_secret_ref\":\"secret://services/twitter\"")
        );
        assert!(!logs[0].contents.contains("raw-token"));
        assert!(!logs[0].contents.contains("raw-bearer-token"));
        assert!(!logs[0].contents.contains("raw-client-secret"));
        assert!(!logs[0].contents.contains("raw-basic-token"));

        let export = database
            .export_system_log()
            .expect("system log should export");
        let exported = fs::read_to_string(&export.path).expect("system log export should read");

        assert!(exported.contains("Worker finished"));
        assert!(exported.contains("Exported system log"));
        assert!(exported.contains("[redacted]"));
        assert!(!exported.contains("raw-token"));
        assert!(!exported.contains("raw-bearer-token"));
        assert!(!exported.contains("raw-client-secret"));
        assert!(!exported.contains("raw-basic-token"));

        let cleared = database
            .clear_system_logs()
            .expect("system logs should clear");

        assert_eq!(cleared.deleted_entries, 2);
        assert!(
            database
                .system_logs()
                .expect("system logs should reload")
                .is_empty()
        );

        fs::remove_file(export.path).expect("temporary system log export should be removed");
        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn reports_system_health_counts() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-health-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: false,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/mastodon/dw-1".to_string(),
                data: None,
            })
            .expect("account should save");
        let connection = database.connection().expect("database should open");
        connection
            .execute(
                "INSERT INTO posts (
                    uuid, status, schedule_status, created_at, updated_at
                 ) VALUES (
                    'failed-post', 3, 2, '2026-06-24T15:00:00Z', '2026-06-24T15:00:00Z'
                 )",
                [],
            )
            .expect("failed post should insert");
        database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 1 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: None,
            })
            .expect("pending job should save");
        let failed_job = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 2 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: None,
            })
            .expect("job should save");
        database
            .fail_job(failed_job.id, "provider failed")
            .expect("job should fail");
        let processing_job = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 3 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: None,
            })
            .expect("processing job should save");
        connection
            .execute(
                "UPDATE job_queue SET status = 'processing' WHERE id = ?1",
                params![processing_job.id],
            )
            .expect("job should be marked processing");
        database
            .save_rate_limit(&RateLimitForm {
                scope: "mixpost-mastodon-api-limit".to_string(),
                retry_after_at: "2026-06-24T16:00:00Z".to_string(),
                payload: None,
            })
            .expect("rate limit should save");

        let counts = database
            .system_health_counts()
            .expect("health counts should load");

        assert_eq!(counts.unauthorized_accounts, 1);
        assert_eq!(counts.failed_posts, 1);
        assert_eq!(counts.pending_jobs, 1);
        assert_eq!(counts.processing_jobs, 1);
        assert_eq!(counts.failed_jobs, 1);
        assert_eq!(counts.rate_limits, 1);

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn builds_dashboard_summary_from_local_state() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-dashboard-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let connection = database.connection().expect("database should open");

        connection
            .execute_batch(
                "INSERT INTO accounts (
                    id, uuid, name, username, provider, provider_id, authorized,
                    access_token_secret_ref, created_at, updated_at
                 )
                 VALUES
                    (1, 'account-x', 'Dust Wave X', 'dustwave', 'twitter', 'dw-x', 1,
                     'secret://accounts/twitter/dw-x', '2026-06-24T12:00:00Z', '2026-06-24T12:00:00Z'),
                    (2, 'account-masto', 'Dust Wave Mastodon', 'dustwave', 'mastodon', 'dw-m', 0,
                     'secret://accounts/mastodon/dw-m', '2026-06-24T12:00:00Z', '2026-06-24T12:00:00Z');

                 INSERT INTO posts (
                    id, uuid, status, schedule_status, scheduled_at, published_at, created_at, updated_at
                 )
                 VALUES
                    (1, 'draft-post', 0, 0, NULL, NULL, '2026-06-24T12:00:00Z', '2026-06-24T12:00:00Z'),
                    (2, 'scheduled-post', 1, 0, '2026-06-25T08:00:00Z', NULL, '2026-06-24T12:00:00Z', '2026-06-24T12:00:00Z'),
                    (3, 'publishing-post', 1, 1, '2026-06-25T09:00:00Z', NULL, '2026-06-24T12:00:00Z', '2026-06-24T12:00:00Z'),
                    (4, 'published-post', 2, 2, '2026-06-23T08:00:00Z', '2026-06-23T08:01:00Z', '2026-06-23T08:00:00Z', '2026-06-23T08:01:00Z'),
                    (5, 'failed-post', 3, 2, '2026-06-22T08:00:00Z', NULL, '2026-06-22T08:00:00Z', '2026-06-24T13:00:00Z');

                 INSERT INTO post_accounts (post_id, account_id)
                 VALUES (2, 1), (3, 1), (5, 2);

                 INSERT INTO post_versions (post_id, account_id, is_original, content_json)
                 VALUES
                    (2, 0, 1, '[{\"body\":\"Scheduled launch\",\"media\":[]}]'),
                    (3, 0, 1, '[{\"body\":\"Publishing now\",\"media\":[]}]'),
                    (5, 0, 1, '[{\"body\":\"Needs retry\",\"media\":[]}]');

                 INSERT INTO job_queue (kind, payload_json, status, attempts, run_at, created_at, updated_at)
                 VALUES
                    ('publish_post', '{\"post_id\":2}', 'pending', 0, '2026-06-25T08:00:00Z', '2026-06-24T12:00:00Z', '2026-06-24T12:00:00Z'),
                    ('publish_post', '{\"post_id\":3}', 'processing', 1, '2026-06-25T09:00:00Z', '2026-06-24T12:00:00Z', '2026-06-24T12:00:00Z'),
                    ('publish_post', '{\"post_id\":5}', 'failed', 1, '2026-06-22T08:00:00Z', '2026-06-22T08:00:00Z', '2026-06-24T13:00:00Z');",
            )
            .expect("fixtures should insert");

        let summary = database
            .dashboard_summary("2026-06-24T16:00:00Z")
            .expect("dashboard should build");

        assert_eq!(summary.generated_at, "2026-06-24T16:00:00Z");
        assert_eq!(summary.accounts.total, 2);
        assert_eq!(summary.accounts.authorized, 1);
        assert_eq!(summary.accounts.unauthorized, 1);
        assert_eq!(summary.accounts.providers, 2);
        assert_eq!(summary.posts.draft, 1);
        assert_eq!(summary.posts.scheduled, 2);
        assert_eq!(summary.posts.publishing, 1);
        assert_eq!(summary.posts.published, 1);
        assert_eq!(summary.posts.failed, 1);
        assert_eq!(summary.jobs.pending, 1);
        assert_eq!(summary.jobs.processing, 1);
        assert_eq!(summary.jobs.failed, 1);
        assert_eq!(summary.providers.len(), 2);
        assert_eq!(summary.upcoming_posts.len(), 2);
        assert_eq!(summary.upcoming_posts[0].preview, "Scheduled launch");
        assert_eq!(summary.upcoming_posts[1].status, "publishing");
        assert_eq!(summary.failed_posts.len(), 1);
        assert_eq!(summary.failed_posts[0].preview, "Needs retry");

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn clears_only_resolved_system_state() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-maintenance-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let completed = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 1 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: None,
            })
            .expect("completed job should save");
        database
            .complete_job(completed.id)
            .expect("job should complete");
        let cancelled = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 2 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: None,
            })
            .expect("cancelled job should save");
        let failed = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 3 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: None,
            })
            .expect("failed job should save");
        database
            .fail_job(failed.id, "provider failed")
            .expect("job should fail");
        database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 4 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: None,
            })
            .expect("pending job should save");

        let connection = database.connection().expect("database should open");
        connection
            .execute(
                "UPDATE job_queue SET status = 'cancelled' WHERE id = ?1",
                params![cancelled.id],
            )
            .expect("job should cancel");
        database
            .save_rate_limit(&RateLimitForm {
                scope: "mixpost-expired-api-limit".to_string(),
                retry_after_at: "2026-06-24T15:59:00Z".to_string(),
                payload: None,
            })
            .expect("expired rate limit should save");
        database
            .save_rate_limit(&RateLimitForm {
                scope: "mixpost-future-api-limit".to_string(),
                retry_after_at: "2026-06-24T17:00:00Z".to_string(),
                payload: None,
            })
            .expect("future rate limit should save");

        let summary = database
            .clear_resolved_system_state("2026-06-24T16:00:00Z")
            .expect("resolved state should clear");

        assert_eq!(summary.completed_jobs_deleted, 1);
        assert_eq!(summary.cancelled_jobs_deleted, 1);
        assert_eq!(summary.expired_rate_limits_cleared, 1);
        assert_eq!(
            count_matching_rows(&connection, "job_queue", "status = 'failed'")
                .expect("failed jobs should count"),
            1
        );
        assert_eq!(
            count_matching_rows(&connection, "job_queue", "status = 'pending'")
                .expect("pending jobs should count"),
            1
        );
        assert_eq!(
            count_rows(&connection, "rate_limits").expect("rate limits should count"),
            1
        );

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn persists_settings() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-settings-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let mut settings = database.settings().expect("default settings should load");

        assert_eq!(settings.timezone, "UTC");
        assert_eq!(settings.time_format, 12);
        assert!(settings.desktop_notifications);

        settings.timezone = "America/Denver".to_string();
        settings.time_format = 24;
        settings.desktop_notifications = false;
        settings.operator_name = "Dust Wave Ops".to_string();
        settings.admin_email = "ops@dustwave.example".to_string();
        settings.default_accounts = vec![1, 2, 3];

        database
            .save_settings(&settings)
            .expect("settings should persist");

        let persisted = database.settings().expect("persisted settings should load");

        assert_eq!(persisted.timezone, "America/Denver");
        assert_eq!(persisted.time_format, 24);
        assert!(!persisted.desktop_notifications);
        assert_eq!(persisted.operator_name, "Dust Wave Ops");
        assert_eq!(persisted.admin_email, "ops@dustwave.example");
        assert_eq!(persisted.default_accounts, vec![1, 2, 3]);

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn saves_updates_and_deletes_accounts() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-account-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let created = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: true,
                avatar_path: Some("avatars/dw.png".to_string()),
                access_token_secret_ref: "secret://accounts/mastodon/dw-1".to_string(),
                data: Some(serde_json::json!({ "server": "mastodon.social" })),
            })
            .expect("account should save");

        assert_eq!(created.name, "Dust Wave");
        assert_eq!(created.provider, "mastodon");
        assert!(created.authorized);

        let updated = database
            .save_account(&AccountForm {
                name: "Dust Wave HQ".to_string(),
                username: Some("dustwavehq".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: false,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/mastodon/dw-1-v2".to_string(),
                data: None,
            })
            .expect("account should update");

        assert_eq!(updated.id, created.id);
        assert_eq!(updated.uuid, created.uuid);
        assert_eq!(updated.name, "Dust Wave HQ");
        assert!(!updated.authorized);
        assert_eq!(database.accounts().expect("accounts should load").len(), 1);

        let connection = database.connection().expect("database should open");
        let data_json = connection
            .query_row(
                "SELECT data_json FROM accounts WHERE id = ?1",
                params![created.id],
                |row| row.get::<_, Option<String>>(0),
            )
            .expect("account data should load");

        assert_eq!(
            data_json.as_deref(),
            Some(r#"{"server":"mastodon.social"}"#)
        );

        assert!(
            database
                .delete_account(&created.uuid)
                .expect("account should delete")
        );
        assert!(
            database
                .accounts()
                .expect("accounts should load")
                .is_empty()
        );

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn rejects_mastodon_refresh_without_connection_metadata() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-mastodon-refresh-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/mastodon/dw-1".to_string(),
                data: None,
            })
            .expect("account should save");
        let error = database
            .refresh_mastodon_account(&account.uuid)
            .expect_err("missing server metadata should fail")
            .to_string();

        assert_eq!(error, "Mastodon account must be connected before refresh");

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn recovers_stale_processing_jobs() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-stale-job-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let job = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 1 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: Some("publish_post:1".to_string()),
            })
            .expect("job should enqueue");
        let reserved = database
            .reserve_due_jobs("2026-06-24T15:00:00Z", 10)
            .expect("job should reserve");

        assert_eq!(reserved.len(), 1);
        assert_eq!(reserved[0].id, job.id);

        let fresh = database
            .recover_stale_processing_jobs("2026-06-24T16:00:00Z", "2000-01-01T00:00:00Z")
            .expect("fresh job should not recover");
        assert_eq!(fresh.requeued_jobs, 0);

        let recovered = database
            .recover_stale_processing_jobs("2026-06-24T16:00:00Z", "2999-01-01T00:00:00Z")
            .expect("stale job should recover");
        assert_eq!(recovered.requeued_jobs, 1);

        let connection = database.connection().expect("database should open");
        let (status, run_at, locked_at, last_error, idempotency_key): (
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = connection
            .query_row(
                "SELECT status, run_at, locked_at, last_error, idempotency_key
                 FROM job_queue
                 WHERE id = ?1",
                params![job.id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("job should reload");

        assert_eq!(status, "pending");
        assert_eq!(run_at, "2026-06-24T16:00:00Z");
        assert!(locked_at.is_none());
        assert_eq!(
            last_error.as_deref(),
            Some("requeued after stale processing lock")
        );
        assert_eq!(idempotency_key.as_deref(), Some("publish_post:1"));

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn loads_local_data_snapshot() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-snapshot-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let connection = database.connection().expect("database should open");

        connection
            .execute_batch(
                "INSERT INTO accounts (
                    id, uuid, name, username, provider, provider_id, data_json, authorized,
                    access_token_secret_ref, created_at, updated_at
                ) VALUES (
                    1, 'account-uuid', 'Dust Wave', 'dustwave', 'mastodon', 'dw-1', NULL, 1,
                    'secret://account/1', '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z'
                );

                INSERT INTO posts (
                    id, uuid, status, schedule_status, scheduled_at, created_at, updated_at
                ) VALUES (
                    1, 'post-uuid', 1, 0, '2026-06-24T15:00:00Z',
                    '2026-06-01T00:00:00Z', '2026-06-02T00:00:00Z'
                );

                INSERT INTO post_accounts (post_id, account_id)
                VALUES (1, 1);

                INSERT INTO post_versions (post_id, account_id, is_original, content_json)
                VALUES (
                    1, 0, 1,
                    '[{\"body\":\"<div>Wave update</div><div>Second line</div>\",\"media\":[1]}]'
                );

                INSERT INTO tags (
                    id, uuid, name, hex_color, created_at, updated_at
                ) VALUES (
                    1, 'tag-uuid', 'Launch', '2f80ed',
                    '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z'
                );

                INSERT INTO tag_post (tag_id, post_id)
                VALUES (1, 1);

                INSERT INTO media (
                    id, uuid, name, mime_type, disk, path, size, size_total,
                    conversions_json, created_at, updated_at
                ) VALUES (
                    1, 'media-uuid', 'launch.png', 'image/png', 'local', 'media/launch.png',
                    100, 150, '{\"thumb\": {}}',
                    '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z'
                );

                INSERT INTO job_queue (
                    id, kind, payload_json, status, attempts, run_at, created_at, updated_at
                ) VALUES (
                    1, 'publish_post', '{\"post_id\":1}', 'pending', 0,
                    '2026-06-24T15:00:00Z', '2026-06-01T00:00:00Z',
                    '2026-06-01T00:00:00Z'
                );",
            )
            .expect("fixtures should insert");

        let snapshot = database
            .local_data_snapshot()
            .expect("snapshot should load");

        assert_eq!(snapshot.accounts.len(), 1);
        assert_eq!(snapshot.accounts[0].provider, "mastodon");
        assert!(snapshot.accounts[0].authorized);

        assert_eq!(snapshot.posts.len(), 1);
        assert_eq!(snapshot.posts[0].status, "scheduled");
        assert_eq!(snapshot.posts[0].account_count, 1);
        assert_eq!(snapshot.posts[0].tag_count, 1);
        assert_eq!(snapshot.posts[0].preview, "Wave update Second line");

        assert_eq!(snapshot.tags.len(), 1);
        assert_eq!(snapshot.tags[0].post_count, 1);

        assert_eq!(snapshot.media.len(), 1);
        assert_eq!(snapshot.media[0].conversion_count, 1);

        assert_eq!(snapshot.jobs.len(), 1);
        assert_eq!(snapshot.jobs[0].kind, "publish_post");

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn mixpost_parity_fixture_covers_representative_workspace_states() {
        let directory = temporary_path("dust-wave-social-parity-fixture-test");
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&path).expect("database should initialize");
        let twitter = database
            .save_account(&AccountForm {
                name: "Dust Wave X".to_string(),
                username: Some("dustwave".to_string()),
                provider: "twitter".to_string(),
                provider_id: "dw-x".to_string(),
                authorized: true,
                avatar_path: Some("avatars/x.png".to_string()),
                access_token_secret_ref: "secret://accounts/twitter/dw-x".to_string(),
                data: Some(serde_json::json!({ "tier": "basic" })),
            })
            .expect("twitter account should save");
        let facebook = database
            .save_account(&AccountForm {
                name: "Dust Wave Page".to_string(),
                username: Some("dustwaveofficial".to_string()),
                provider: "facebook_page".to_string(),
                provider_id: "dw-fb-page".to_string(),
                authorized: true,
                avatar_path: Some("avatars/facebook.png".to_string()),
                access_token_secret_ref: "secret://accounts/facebook_page/dw-page".to_string(),
                data: Some(serde_json::json!({ "page_id": "dw-fb-page" })),
            })
            .expect("facebook page account should save");
        let mastodon = database
            .save_account(&AccountForm {
                name: "Dust Wave Mastodon".to_string(),
                username: Some("dustwave@social.example".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-mastodon".to_string(),
                authorized: false,
                avatar_path: Some("avatars/mastodon.png".to_string()),
                access_token_secret_ref: "secret://accounts/mastodon/dw".to_string(),
                data: Some(serde_json::json!({ "server": "social.example" })),
            })
            .expect("mastodon account should save");
        let launch = database
            .create_tag(&TagForm {
                name: "Launch".to_string(),
                hex_color: "#2f80ed".to_string(),
            })
            .expect("launch tag should create");
        let evergreen = database
            .create_tag(&TagForm {
                name: "Evergreen".to_string(),
                hex_color: "#22ff88".to_string(),
            })
            .expect("evergreen tag should create");
        let image = database
            .create_media(&MediaForm {
                name: "launch-cover.png".to_string(),
                mime_type: "image/png".to_string(),
                disk: "local".to_string(),
                path: "media/launch-cover.png".to_string(),
                size: 120_000,
                size_total: 150_000,
                data: Some(serde_json::json!({ "source": "upload" })),
                conversions: Some(serde_json::json!({
                    "thumb": { "path": "media/conversions/launch-cover-thumb.png" }
                })),
            })
            .expect("image media should create");
        let gif = database
            .create_media(&MediaForm {
                name: "wave-loop.gif".to_string(),
                mime_type: "image/gif".to_string(),
                disk: "local".to_string(),
                path: "media/wave-loop.gif".to_string(),
                size: 240_000,
                size_total: 240_000,
                data: Some(serde_json::json!({ "source": "upload" })),
                conversions: None,
            })
            .expect("gif media should create");

        let draft_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [],
            "tags": [evergreen.id],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Draft positioning idea",
                            "media": [image.id]
                        }
                    ]
                }
            ]
        }))
        .expect("draft form should deserialize");
        let draft = database
            .create_draft_post(&draft_form)
            .expect("draft should create");

        let scheduled_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [twitter.id, facebook.id, mastodon.id],
            "tags": [launch.id, evergreen.id],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Launch wave across every channel",
                            "media": [image.id]
                        }
                    ]
                },
                {
                    "account_id": twitter.id,
                    "is_original": false,
                    "content": [
                        {
                            "body": "Launch wave for X",
                            "media": [gif.id]
                        }
                    ]
                },
                {
                    "account_id": mastodon.id,
                    "is_original": false,
                    "content": [
                        {
                            "body": "Launch wave for Mastodon",
                            "media": [image.id]
                        }
                    ]
                }
            ]
        }))
        .expect("scheduled form should deserialize");
        let scheduled = database
            .create_draft_post(&scheduled_form)
            .expect("scheduled source should create");
        let scheduled = database
            .schedule_post(
                &scheduled.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect("post should schedule");

        let published_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [twitter.id],
            "tags": [launch.id],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Published recap",
                            "media": [gif.id]
                        }
                    ]
                }
            ]
        }))
        .expect("published form should deserialize");
        let published = database
            .create_draft_post(&published_form)
            .expect("published source should create");

        let failed_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [facebook.id],
            "tags": [evergreen.id],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Failed campaign asset",
                            "media": [image.id]
                        }
                    ]
                }
            ]
        }))
        .expect("failed form should deserialize");
        let failed = database
            .create_draft_post(&failed_form)
            .expect("failed source should create");

        {
            let connection = database.connection().expect("database should open");
            connection
                .execute(
                    "UPDATE posts
                     SET status = 2,
                         schedule_status = 2,
                         scheduled_at = '2026-06-23T15:00:00Z',
                         published_at = '2026-06-23T15:01:00Z'
                     WHERE id = ?1",
                    params![published.id],
                )
                .expect("published fixture should update");
            connection
                .execute(
                    "UPDATE posts
                     SET status = 3,
                         schedule_status = 2,
                         scheduled_at = '2026-06-22T15:00:00Z'
                     WHERE id = ?1",
                    params![failed.id],
                )
                .expect("failed fixture should update");
            connection
                .execute(
                    "UPDATE post_accounts
                     SET errors_json = '[\"Graph API rejected media\"]'
                     WHERE post_id = ?1 AND account_id = ?2",
                    params![failed.id, facebook.id],
                )
                .expect("failed fixture error should update");
        }

        database
            .save_rate_limit(&RateLimitForm {
                scope: account_rate_limit_scope(twitter.id),
                retry_after_at: "2026-06-24T16:00:00Z".to_string(),
                payload: Some(serde_json::json!({ "source": "provider" })),
            })
            .expect("rate limit should save");

        let validation = database
            .validate_post(&scheduled.uuid)
            .expect("scheduled post should validate");
        assert!(validation.valid);

        let dashboard = database
            .dashboard_summary("2026-06-24T14:00:00Z")
            .expect("dashboard should build");
        assert_eq!(dashboard.accounts.total, 3);
        assert_eq!(dashboard.accounts.authorized, 2);
        assert_eq!(dashboard.accounts.unauthorized, 1);
        assert_eq!(dashboard.accounts.providers, 3);
        assert_eq!(dashboard.posts.draft, 1);
        assert_eq!(dashboard.posts.scheduled, 1);
        assert_eq!(dashboard.posts.published, 1);
        assert_eq!(dashboard.posts.failed, 1);
        assert_eq!(dashboard.jobs.pending, 1);
        assert_eq!(dashboard.upcoming_posts.len(), 1);
        assert_eq!(
            dashboard.upcoming_posts[0].preview,
            "Launch wave across every channel"
        );
        assert_eq!(dashboard.failed_posts.len(), 1);
        assert_eq!(dashboard.failed_posts[0].preview, "Failed campaign asset");
        let provider_ids = dashboard
            .providers
            .iter()
            .map(|provider| provider.provider.as_str())
            .collect::<Vec<_>>();
        assert!(provider_ids.contains(&"twitter"));
        assert!(provider_ids.contains(&"facebook_page"));
        assert!(provider_ids.contains(&"mastodon"));

        let snapshot = database
            .local_data_snapshot()
            .expect("snapshot should load");
        assert_eq!(snapshot.accounts.len(), 3);
        assert_eq!(snapshot.posts.len(), 4);
        assert_eq!(snapshot.media.len(), 2);
        assert_eq!(snapshot.tags.len(), 2);
        assert_eq!(snapshot.jobs.len(), 1);
        assert_eq!(snapshot.rate_limits.len(), 1);
        let snapshot_statuses = snapshot
            .posts
            .iter()
            .map(|post| post.status.as_str())
            .collect::<Vec<_>>();
        assert!(snapshot_statuses.contains(&"draft"));
        assert!(snapshot_statuses.contains(&"scheduled"));
        assert!(snapshot_statuses.contains(&"published"));
        assert!(snapshot_statuses.contains(&"failed"));

        let detail = database
            .post_detail(&scheduled.uuid)
            .expect("scheduled detail should load");
        assert_eq!(detail.status, "scheduled");
        assert_eq!(detail.accounts, vec![twitter.id, facebook.id, mastodon.id]);
        assert_eq!(detail.tags, vec![launch.id, evergreen.id]);
        assert_eq!(detail.versions.len(), 3);
        assert_eq!(detail.versions[1].content[0].body, "Launch wave for X");

        let scheduled_result = database
            .query_posts(&PostQueryRequest {
                status: Some("scheduled".to_string()),
                exclude_status: None,
                keyword: Some("mastodon".to_string()),
                accounts: vec![mastodon.id],
                tags: vec![launch.id],
                calendar_type: None,
                date: None,
                limit: Some(20),
                page: None,
            })
            .expect("scheduled post query should run");
        assert_eq!(scheduled_result.total, 1);
        assert_eq!(scheduled_result.items[0].uuid, scheduled.uuid);
        assert_eq!(scheduled_result.items[0].account_count, 3);
        assert_eq!(scheduled_result.items[0].tag_count, 2);
        assert_eq!(scheduled_result.items[0].media.len(), 1);

        let failed_result = database
            .query_posts(&PostQueryRequest {
                status: Some("failed".to_string()),
                exclude_status: None,
                keyword: Some("campaign".to_string()),
                accounts: vec![facebook.id],
                tags: vec![evergreen.id],
                calendar_type: None,
                date: None,
                limit: Some(20),
                page: None,
            })
            .expect("failed post query should run");
        assert!(failed_result.has_failed_posts);
        assert_eq!(failed_result.total, 1);
        assert_eq!(
            failed_result.items[0].failure_errors,
            vec!["Graph API rejected media".to_string()]
        );

        let media_library = database.media_library().expect("media library should load");
        assert_eq!(media_library.len(), 2);
        let media_types = media_library
            .iter()
            .map(|media| media.media_type.as_str())
            .collect::<Vec<_>>();
        assert!(media_types.contains(&"image"));
        assert!(media_types.contains(&"gif"));
        assert_eq!(
            media_library
                .iter()
                .find(|media| media.media_type == "image")
                .expect("image media should be present")
                .conversion_count,
            1
        );

        assert_eq!(draft.status, "draft");

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn providerless_mvp_fixture_covers_schedule_import_retry_reports_and_backups() {
        let directory = temporary_path("dust-wave-social-providerless-mvp-test");
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&path).expect("database should initialize");

        let twitter = database
            .save_account(&AccountForm {
                name: "Dust Wave X".to_string(),
                username: Some("dustwave".to_string()),
                provider: "twitter".to_string(),
                provider_id: "dw-x".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/twitter/dw-x/access_token".to_string(),
                data: Some(serde_json::json!({ "auth": "oauth2_pkce" })),
            })
            .expect("twitter account should save");
        let facebook = database
            .save_account(&AccountForm {
                name: "Dust Wave Facebook".to_string(),
                username: Some("dustwaveofficial".to_string()),
                provider: "facebook_page".to_string(),
                provider_id: "dw-facebook".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref:
                    "secret://accounts/facebook_page/dw-facebook/page_access_token".to_string(),
                data: Some(serde_json::json!({ "auth": "facebook_user" })),
            })
            .expect("facebook page account should save");
        let mastodon = database
            .save_account(&AccountForm {
                name: "Dust Wave Mastodon".to_string(),
                username: Some("dustwave@social.example".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-mastodon".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/mastodon/dw-mastodon/access_token"
                    .to_string(),
                data: Some(serde_json::json!({ "server": "https://social.example" })),
            })
            .expect("mastodon account should save");
        let disconnected = database
            .save_account(&AccountForm {
                name: "Needs Reconnect".to_string(),
                username: Some("needsreconnect".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "needs-reconnect".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/mastodon/needs-reconnect/access_token"
                    .to_string(),
                data: None,
            })
            .expect("disconnected account should save");
        let launch = database
            .create_tag(&TagForm {
                name: "Launch".to_string(),
                hex_color: "#2f80ed".to_string(),
            })
            .expect("tag should create");
        let media = database
            .create_media(&MediaForm {
                name: "launch.png".to_string(),
                mime_type: "image/png".to_string(),
                disk: "local".to_string(),
                path: "media/launch.png".to_string(),
                size: 42,
                size_total: 42,
                data: Some(serde_json::json!({ "source": "fixture" })),
                conversions: None,
            })
            .expect("media should create");

        let scheduled_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [twitter.id, facebook.id, mastodon.id],
            "tags": [launch.id],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Providerless MVP launch fixture",
                            "media": [media.id]
                        }
                    ]
                },
                {
                    "account_id": twitter.id,
                    "is_original": false,
                    "content": [
                        {
                            "body": "Providerless MVP launch fixture for X",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("scheduled form should deserialize");
        let scheduled = database
            .create_draft_post(&scheduled_form)
            .expect("scheduled post should create");
        let scheduled = database
            .schedule_post(
                &scheduled.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect("post should schedule");

        let failing_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [disconnected.id],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Reconnect before publishing",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("failing form should deserialize");
        let failing = database
            .create_draft_post(&failing_form)
            .expect("failing post should create");
        database
            .schedule_post(
                &failing.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T14:00:00Z".to_string(),
                },
            )
            .expect("failing post should schedule");

        let validation = database
            .validate_post(&scheduled.uuid)
            .expect("scheduled post should validate");
        assert!(validation.valid);

        let failed_publish = database
            .run_due_jobs("2026-06-24T14:00:00Z", 10)
            .expect("worker should run disconnected publish job");
        assert_eq!(failed_publish.reserved, 1);
        assert_eq!(failed_publish.failed, 1);
        assert!(
            failed_publish.outcomes[0]
                .detail
                .contains("must be connected")
        );

        let retried = database
            .retry_failed_post(
                &failing.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T16:00:00Z".to_string(),
                },
            )
            .expect("failed post should retry");
        assert_eq!(retried.status, "scheduled");
        assert_eq!(retried.schedule_status, "pending");

        let batch = database
            .enqueue_all_account_imports("2026-06-24T13:00:00Z")
            .expect("account imports should enqueue");
        assert_eq!(batch.requested_accounts, 4);
        assert_eq!(batch.eligible_accounts, 4);
        assert_eq!(batch.queued_jobs, 4);
        assert_eq!(batch.skipped_unsupported, 0);
        assert_eq!(batch.skipped_unauthorized, 0);

        {
            let mut connection = database.connection().expect("database should open");
            let transaction = connection.transaction().expect("transaction should start");
            let twitter_imported = import_twitter_post_rows(
                &transaction,
                twitter.id,
                &[serde_json::json!({
                    "id": "tweet-1",
                    "created_at": "2026-06-24T12:00:00Z",
                    "text": "Launch tweet",
                    "public_metrics": {
                        "like_count": 7,
                        "retweet_count": 3,
                        "reply_count": 2,
                        "impression_count": 120
                    }
                })],
            )
            .expect("twitter rows should import");
            let twitter_metric_days = process_twitter_metric_days(&transaction, twitter.id)
                .expect("twitter metrics should process");
            let facebook_insights = import_facebook_insight_rows(
                &transaction,
                facebook.id,
                &[
                    serde_json::json!({
                        "name": "page_post_engagements",
                        "values": [{"value": 11, "end_time": "2026-06-24T00:00:00+0000"}]
                    }),
                    serde_json::json!({
                        "name": "page_posts_impressions",
                        "values": [{"value": 220, "end_time": "2026-06-24T00:00:00+0000"}]
                    }),
                ],
            )
            .expect("facebook insight rows should import");
            let mastodon_imported = import_mastodon_status_rows(
                &transaction,
                mastodon.id,
                &[serde_json::json!({
                    "id": "status-1",
                    "created_at": "2026-06-24T12:00:00Z",
                    "content": "Launch toot",
                    "replies_count": 4,
                    "reblogs_count": 5,
                    "favourites_count": 6
                })],
            )
            .expect("mastodon rows should import");
            let mastodon_metric_days = process_mastodon_metric_days(&transaction, mastodon.id)
                .expect("mastodon metrics should process");
            transaction.commit().expect("fixture imports should commit");

            assert_eq!(twitter_imported, 1);
            assert_eq!(twitter_metric_days, 1);
            assert_eq!(facebook_insights, 2);
            assert_eq!(mastodon_imported, 1);
            assert_eq!(mastodon_metric_days, 1);
        }

        for (account_id, total) in [
            (twitter.id, 1_200),
            (facebook.id, 2_400),
            (mastodon.id, 800),
        ] {
            database
                .save_audience(&AudienceForm {
                    account_id,
                    date: "2026-06-24".to_string(),
                    total,
                })
                .expect("audience should save");
        }

        let end_date = NaiveDate::from_ymd_opt(2026, 6, 24).expect("date should be valid");
        let twitter_report = database
            .account_report_for_end_date(twitter.id, "30_days", 30, end_date)
            .expect("twitter report should build");
        assert_eq!(twitter_report.provider, "twitter");
        assert_eq!(
            twitter_report
                .metrics
                .iter()
                .find(|metric| metric.key == "likes")
                .expect("likes metric should exist")
                .value,
            7
        );
        assert_eq!(
            twitter_report
                .metrics
                .iter()
                .find(|metric| metric.key == "impressions")
                .expect("impressions metric should exist")
                .value,
            120
        );
        assert!(twitter_report.audience.values.contains(&Some(1_200)));

        let facebook_report = database
            .account_report_for_end_date(facebook.id, "30_days", 30, end_date)
            .expect("facebook report should build");
        assert_eq!(
            facebook_report
                .metrics
                .iter()
                .find(|metric| metric.key == "page_posts_impressions")
                .expect("impression metric should exist")
                .value,
            220
        );

        let mastodon_report = database
            .account_report_for_end_date(mastodon.id, "30_days", 30, end_date)
            .expect("mastodon report should build");
        assert_eq!(
            mastodon_report
                .metrics
                .iter()
                .find(|metric| metric.key == "favourites")
                .expect("favourites metric should exist")
                .value,
            6
        );

        let post_result = database
            .query_posts(&PostQueryRequest {
                status: Some("scheduled".to_string()),
                exclude_status: None,
                keyword: Some("providerless".to_string()),
                accounts: vec![twitter.id],
                tags: vec![launch.id],
                calendar_type: None,
                date: None,
                limit: Some(20),
                page: None,
            })
            .expect("post query should run");
        assert_eq!(post_result.total, 1);
        assert_eq!(post_result.items[0].uuid, scheduled.uuid);

        let backup = database
            .export_local_backup()
            .expect("providerless fixture backup should export");
        let manifest = fs::read_to_string(&backup.manifest_path).expect("manifest should read");
        assert!(manifest.contains("\"product\": \"Dust Wave Social\""));
        assert!(manifest.contains("\"included\": false"));
        assert!(!manifest.contains("dw-x-access-token"));
        assert!(!manifest.contains("page-access-token"));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn exports_local_backup_with_database_media_and_manifest() {
        let directory = std::env::temp_dir().join(format!(
            "dust-wave-social-backup-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&path).expect("database should initialize");
        let media_directory = directory.join("media");
        let media_path = media_directory.join("sample.txt");

        fs::create_dir_all(&media_directory).expect("media directory should be created");
        fs::write(&media_path, b"backup-media").expect("media fixture should write");

        let backup = database
            .export_local_backup()
            .expect("backup should export");
        let manifest =
            fs::read_to_string(&backup.manifest_path).expect("manifest should be readable");

        assert!(Path::new(&backup.path).exists());
        assert!(Path::new(&backup.database_path).exists());
        assert!(Path::new(&backup.media_path).join("sample.txt").exists());
        assert!(manifest.contains("\"included\": false"));
        assert!(manifest.contains("OS keychain service credentials"));
        assert_eq!(backup.media_files, 1);
        assert!(backup.bytes > 0);

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn restores_local_backup_with_safety_copy() {
        let directory = std::env::temp_dir().join(format!(
            "dust-wave-social-restore-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&path).expect("database should initialize");
        let media_directory = directory.join("media");
        let media_path = media_directory.join("sample.txt");

        fs::create_dir_all(&media_directory).expect("media directory should be created");
        fs::write(&media_path, b"original-media").expect("media fixture should write");
        database
            .save_account(&AccountForm {
                name: "Original".to_string(),
                username: Some("original".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "original-1".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/mastodon/original-1".to_string(),
                data: None,
            })
            .expect("original account should save");

        let backup = database
            .export_local_backup()
            .expect("backup should export");

        database
            .save_account(&AccountForm {
                name: "Replacement".to_string(),
                username: Some("replacement".to_string()),
                provider: "twitter".to_string(),
                provider_id: "replacement-1".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/twitter/replacement-1".to_string(),
                data: None,
            })
            .expect("replacement account should save");
        fs::write(&media_path, b"changed-media").expect("media should change");
        fs::write(media_directory.join("extra.txt"), b"extra").expect("extra media should write");

        let restored = database
            .restore_local_backup(&LocalBackupRestoreForm {
                backup_path: backup.path.clone(),
            })
            .expect("backup should restore");
        let snapshot = database
            .local_data_snapshot()
            .expect("snapshot should reload after restore");

        assert_eq!(snapshot.accounts.len(), 1);
        assert_eq!(snapshot.accounts[0].name, "Original");
        assert!(Path::new(&restored.safety_backup_path).exists());
        assert_eq!(
            fs::read_to_string(&media_path).expect("restored media should read"),
            "original-media"
        );
        assert!(!media_directory.join("extra.txt").exists());
        assert_eq!(restored.restored_media_files, 1);
        assert!(restored.restored_bytes > 0);

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn rejects_local_backup_with_wrong_product_manifest() {
        let directory = temporary_path("dust-wave-social-restore-product-test");
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&path).expect("database should initialize");
        let backup = database
            .export_local_backup()
            .expect("backup should export");
        let mut manifest = backup_manifest(&backup);

        manifest["product"] = serde_json::json!("Mixpost");
        write_backup_manifest(&backup, &manifest);

        let error = database
            .restore_local_backup(&LocalBackupRestoreForm {
                backup_path: backup.path,
            })
            .expect_err("wrong-product backup should not restore")
            .to_string();

        assert!(error.contains("product is not Dust Wave Social"));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn rejects_local_backup_manifest_that_claims_included_secrets() {
        let directory = temporary_path("dust-wave-social-restore-secrets-test");
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&path).expect("database should initialize");
        let backup = database
            .export_local_backup()
            .expect("backup should export");
        let mut manifest = backup_manifest(&backup);

        manifest["secrets"]["included"] = serde_json::json!(true);
        write_backup_manifest(&backup, &manifest);

        let error = database
            .restore_local_backup(&LocalBackupRestoreForm {
                backup_path: backup.path,
            })
            .expect_err("secret-bearing backup should not restore")
            .to_string();

        assert!(error.contains("secrets are excluded"));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn rejects_local_backup_with_corrupt_manifest() {
        let directory = temporary_path("dust-wave-social-restore-corrupt-manifest-test");
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&path).expect("database should initialize");
        let backup = database
            .export_local_backup()
            .expect("backup should export");

        fs::write(&backup.manifest_path, b"{ not json").expect("manifest should corrupt");

        let error = database
            .restore_local_backup(&LocalBackupRestoreForm {
                backup_path: backup.path,
            })
            .expect_err("corrupt manifest should not restore")
            .to_string();

        assert!(error.contains("json error"));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn rejects_local_backup_without_database_file() {
        let directory = temporary_path("dust-wave-social-restore-database-test");
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&path).expect("database should initialize");
        let backup = database
            .export_local_backup()
            .expect("backup should export");

        fs::remove_file(&backup.database_path).expect("backup database should be removed");

        let error = database
            .restore_local_backup(&LocalBackupRestoreForm {
                backup_path: backup.path,
            })
            .expect_err("backup without database should not restore")
            .to_string();

        assert!(error.contains("backup database dust-wave-social.sqlite3 is required"));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn rejects_local_backup_without_media_folder() {
        let directory = temporary_path("dust-wave-social-restore-media-test");
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&path).expect("database should initialize");
        let backup = database
            .export_local_backup()
            .expect("backup should export");

        fs::remove_dir_all(&backup.media_path).expect("backup media folder should be removed");

        let error = database
            .restore_local_backup(&LocalBackupRestoreForm {
                backup_path: backup.path,
            })
            .expect_err("backup without media folder should not restore")
            .to_string();

        assert!(error.contains("backup media folder is required"));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn rejects_local_backup_with_newer_schema_manifest() {
        let directory = std::env::temp_dir().join(format!(
            "dust-wave-social-restore-schema-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&path).expect("database should initialize");
        let backup = database
            .export_local_backup()
            .expect("backup should export");
        let manifest_path = Path::new(&backup.manifest_path);
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(manifest_path).expect("manifest should be readable"))
                .expect("manifest should be json");

        manifest["schema_version"] = serde_json::json!(i64::from(CURRENT_SCHEMA_VERSION) + 1);
        fs::write(
            manifest_path,
            serde_json::to_vec_pretty(&manifest).expect("manifest should serialize"),
        )
        .expect("manifest should update");

        let error = database
            .restore_local_backup(&LocalBackupRestoreForm {
                backup_path: backup.path,
            })
            .expect_err("newer backup should not restore")
            .to_string();

        assert!(error.contains("newer than this app supports"));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn creates_updates_and_deletes_tags() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-tag-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let created = database
            .create_tag(&TagForm {
                name: " Launch ".to_string(),
                hex_color: "#2f8".to_string(),
            })
            .expect("tag should create");

        assert_eq!(created.name, "Launch");
        assert_eq!(created.hex_color, "22ff88");
        assert_eq!(database.tags().expect("tags should load").len(), 1);

        let updated = database
            .update_tag(
                &created.uuid,
                &TagForm {
                    name: "Campaign".to_string(),
                    hex_color: "#123456".to_string(),
                },
            )
            .expect("tag should update");

        assert_eq!(updated.name, "Campaign");
        assert_eq!(updated.hex_color, "123456");

        assert!(
            database
                .delete_tag(&created.uuid)
                .expect("tag should delete")
        );
        assert!(database.tags().expect("tags should load").is_empty());

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn saves_and_lists_services() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-service-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let created = database
            .save_service(&ServiceForm {
                name: "mastodon".to_string(),
                configuration_secret_ref: "secret://services/mastodon".to_string(),
                configuration: Some(serde_json::json!({ "server": "mastodon.social" })),
                active: true,
            })
            .expect("service should save");

        assert_eq!(created.name, "mastodon");
        assert!(created.active);

        let updated = database
            .save_service(&ServiceForm {
                name: "mastodon".to_string(),
                configuration_secret_ref: "secret://services/mastodon-v2".to_string(),
                configuration: None,
                active: false,
            })
            .expect("service should update");

        assert_eq!(
            updated.configuration_secret_ref,
            "secret://services/mastodon-v2"
        );
        assert_eq!(
            updated
                .configuration
                .get("server")
                .and_then(|value| value.as_str()),
            Some("mastodon.social")
        );
        assert!(!updated.active);

        let snapshot = database
            .local_data_snapshot()
            .expect("snapshot should load");
        assert_eq!(snapshot.services.len(), 1);
        assert_eq!(database.services().expect("services should load").len(), 1);

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn creates_lists_and_deletes_media_records() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-media-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let created = database
            .create_media(&MediaForm {
                name: "launch.png".to_string(),
                mime_type: "image/png".to_string(),
                disk: "local".to_string(),
                path: "media/launch.png".to_string(),
                size: 100,
                size_total: 150,
                data: Some(serde_json::json!({ "source": "upload" })),
                conversions: Some(serde_json::json!({ "thumb": {} })),
            })
            .expect("media should create");

        assert_eq!(created.name, "launch.png");
        assert_eq!(created.conversion_count, 1);
        assert_eq!(database.media().expect("media should load").len(), 1);

        let snapshot = database
            .local_data_snapshot()
            .expect("snapshot should load");
        assert_eq!(snapshot.media.len(), 1);

        assert!(
            database
                .delete_media(&created.uuid)
                .expect("media should delete")
        );
        assert!(database.media().expect("media should load").is_empty());

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn queries_media_library_with_keyword_type_and_limit_filters() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-media-library-query-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let image = database
            .create_media(&MediaForm {
                name: "Launch photo.png".to_string(),
                mime_type: "image/png".to_string(),
                disk: "local".to_string(),
                path: "media/launch-photo.png".to_string(),
                size: 100,
                size_total: 100,
                data: None,
                conversions: None,
            })
            .expect("image media should create");
        let gif = database
            .create_media(&MediaForm {
                name: "Reaction.gif".to_string(),
                mime_type: "image/gif".to_string(),
                disk: "local".to_string(),
                path: "media/reaction.gif".to_string(),
                size: 200,
                size_total: 200,
                data: None,
                conversions: None,
            })
            .expect("gif media should create");
        let video = database
            .create_media(&MediaForm {
                name: "Trailer.mp4".to_string(),
                mime_type: "video/mp4".to_string(),
                disk: "local".to_string(),
                path: "media/trailer.mp4".to_string(),
                size: 300,
                size_total: 300,
                data: None,
                conversions: None,
            })
            .expect("video media should create");
        let file = database
            .create_media(&MediaForm {
                name: "Press kit.pdf".to_string(),
                mime_type: "application/pdf".to_string(),
                disk: "local".to_string(),
                path: "media/press-kit.pdf".to_string(),
                size: 400,
                size_total: 400,
                data: None,
                conversions: None,
            })
            .expect("file media should create");

        let keyword_results = database
            .query_media_library(&MediaLibraryRequest {
                keyword: Some("launch".to_string()),
                media_type: None,
                limit: Some(10),
            })
            .expect("keyword query should load");
        assert_eq!(keyword_results.len(), 1);
        assert_eq!(keyword_results[0].id, image.id);

        let image_results = database
            .query_media_library(&MediaLibraryRequest {
                keyword: None,
                media_type: Some("image".to_string()),
                limit: Some(10),
            })
            .expect("image query should load");
        assert_eq!(image_results.len(), 1);
        assert_eq!(image_results[0].id, image.id);

        let gif_results = database
            .query_media_library(&MediaLibraryRequest {
                keyword: None,
                media_type: Some("gif".to_string()),
                limit: Some(10),
            })
            .expect("gif query should load");
        assert_eq!(gif_results.len(), 1);
        assert_eq!(gif_results[0].id, gif.id);
        assert_eq!(gif_results[0].media_type, "gif");

        let video_results = database
            .query_media_library(&MediaLibraryRequest {
                keyword: None,
                media_type: Some("video".to_string()),
                limit: Some(10),
            })
            .expect("video query should load");
        assert_eq!(video_results.len(), 1);
        assert_eq!(video_results[0].id, video.id);
        assert!(video_results[0].is_video);

        let file_results = database
            .query_media_library(&MediaLibraryRequest {
                keyword: None,
                media_type: Some("file".to_string()),
                limit: Some(10),
            })
            .expect("file query should load");
        assert_eq!(file_results.len(), 1);
        assert_eq!(file_results[0].id, file.id);

        let limited_results = database
            .query_media_library(&MediaLibraryRequest {
                keyword: None,
                media_type: None,
                limit: Some(2),
            })
            .expect("limited query should load");
        assert_eq!(limited_results.len(), 2);

        let invalid_error = database
            .query_media_library(&MediaLibraryRequest {
                keyword: None,
                media_type: Some("audio".to_string()),
                limit: None,
            })
            .expect_err("unsupported type should fail")
            .to_string();
        assert_eq!(
            invalid_error,
            "media_type must be image, gif, video, or file"
        );

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn imports_local_media_files_into_app_storage() {
        let directory = std::env::temp_dir().join(format!(
            "dust-wave-social-media-import-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("temporary directory should be created");

        let source_path = directory.join("sample.png");
        let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([47, 128, 237, 255]));
        image
            .save(&source_path)
            .expect("source media file should be written");
        let source_size = i64::try_from(
            fs::metadata(&source_path)
                .expect("source media metadata should load")
                .len(),
        )
        .expect("source media size should fit");

        let database_path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&database_path).expect("database should initialize");
        let imported = database
            .import_media_file(&MediaImportForm {
                source_path: source_path.display().to_string(),
                name: None,
            })
            .expect("media file should import");

        assert_eq!(imported.name, "sample.png");
        assert_eq!(imported.mime_type, "image/png");
        assert_eq!(imported.disk, "local");
        assert!(imported.path.starts_with("media/"));
        assert_eq!(imported.size, source_size);
        assert!(imported.size_total > imported.size);
        assert_eq!(imported.conversion_count, 1);
        assert_eq!(database.media().expect("media should load").len(), 1);

        let copied_path = directory.join(&imported.path);
        assert_eq!(
            fs::read(copied_path).expect("copied media should be readable"),
            fs::read(&source_path).expect("source media should be readable")
        );

        let connection = database.connection().expect("database should open");
        let conversions_json: String = connection
            .query_row(
                "SELECT conversions_json FROM media WHERE uuid = ?1",
                params![&imported.uuid],
                |row| row.get(0),
            )
            .expect("conversions should load");
        let conversions: serde_json::Value =
            serde_json::from_str(&conversions_json).expect("conversions should be json");
        let thumb_path = conversions[0]["path"]
            .as_str()
            .expect("thumb path should exist")
            .to_string();

        assert_eq!(conversions[0]["engine"], "ImageResize");
        assert_eq!(conversions[0]["disk"], "local");
        assert_eq!(conversions[0]["name"], "thumb");
        assert!(directory.join(&thumb_path).exists());

        let library = database.media_library().expect("media library should load");
        assert_eq!(library.len(), 1);
        assert_eq!(library[0].id, imported.id);
        assert_eq!(library[0].media_type, "image");
        assert!(!library[0].is_video);
        let expected_url = directory.join(&imported.path).display().to_string();
        let expected_thumb_url = directory.join(&thumb_path).display().to_string();
        assert_eq!(library[0].url.as_deref(), Some(expected_url.as_str()));
        assert_eq!(
            library[0].thumb_url.as_deref(),
            Some(expected_thumb_url.as_str())
        );

        assert!(
            database
                .delete_media(&imported.uuid)
                .expect("imported media should delete")
        );
        assert!(!directory.join(&imported.path).exists());
        assert!(!directory.join(&thumb_path).exists());
        assert!(database.media().expect("media should load").is_empty());

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn downloads_external_media_from_file_urls_into_app_storage() {
        let directory = std::env::temp_dir().join(format!(
            "dust-wave-social-media-download-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("temporary directory should be created");

        let source_path = directory.join("external.gif");
        fs::write(&source_path, [71, 73, 70, 56, 57, 97])
            .expect("source media file should be written");

        let database_path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&database_path).expect("database should initialize");
        let source_url = Url::from_file_path(&source_path)
            .expect("file URL should build")
            .to_string();
        let downloaded = database
            .download_external_media(&MediaDownloadForm {
                url: source_url.clone(),
                name: Some("Downloaded GIF".to_string()),
                source: Some("url".to_string()),
                download_data: Some(serde_json::json!({ "provider": "manual" })),
            })
            .expect("external media should download");

        assert_eq!(downloaded.name, "Downloaded GIF");
        assert_eq!(downloaded.mime_type, "image/gif");
        assert_eq!(downloaded.disk, "local");
        assert!(downloaded.path.starts_with("media/"));
        assert_eq!(downloaded.size, 6);
        assert_eq!(downloaded.size_total, 6);
        assert_eq!(database.media().expect("media should load").len(), 1);
        assert_eq!(
            fs::read(directory.join(&downloaded.path))
                .expect("downloaded media should be readable"),
            vec![71, 73, 70, 56, 57, 97]
        );

        let connection = database.connection().expect("database should open");
        let data_json: String = connection
            .query_row(
                "SELECT data_json FROM media WHERE uuid = ?1",
                params![downloaded.uuid],
                |row| row.get(0),
            )
            .expect("media data should load");
        let data: serde_json::Value =
            serde_json::from_str(&data_json).expect("media data should be json");

        assert_eq!(data["source"], "url");
        assert_eq!(data["external_url"], source_url);
        assert_eq!(data["original_name"], "external.gif");
        assert_eq!(
            data["download_data"],
            serde_json::json!({ "provider": "manual" })
        );

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn rejects_klipy_external_media_downloads_into_app_storage() {
        let directory = std::env::temp_dir().join(format!(
            "dust-wave-social-klipy-download-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("temporary directory should be created");

        let database_path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&database_path).expect("database should initialize");
        let error = database
            .download_external_media(&MediaDownloadForm {
                url: "https://cdn.klipy.com/example.gif".to_string(),
                name: Some("Klipy GIF".to_string()),
                source: Some("gifs".to_string()),
                download_data: Some(serde_json::json!({ "provider": "klipy" })),
            })
            .expect_err("klipy media should not be permanently imported");

        assert!(
            error
                .to_string()
                .contains("Klipy GIFs cannot be saved to the reusable media library")
        );
        assert!(database.media().expect("media should load").is_empty());

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn fetches_klipy_external_media_to_transient_publish_asset() {
        let (address, handle) = serve_test_media_response("image/gif", b"GIF89a");
        let directory = temporary_path("dust-wave-social-klipy-transient-test");
        fs::create_dir_all(&directory).expect("temporary directory should be created");

        let database_path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&database_path).expect("database should initialize");
        let media = ExternalMediaItem {
            id: "wave".to_string(),
            name: "Wave".to_string(),
            mime_type: "image/gif".to_string(),
            media_type: "gif".to_string(),
            url: format!("http://{address}/wave.gif"),
            thumb_url: format!("http://{address}/wave-thumb.gif"),
            is_video: false,
            credit_url: None,
            download_data: Some(serde_json::json!({ "provider": "klipy" })),
        };
        let asset = database
            .transient_external_media_asset(&media)
            .expect("klipy reference should fetch to transient asset");

        assert_eq!(asset.id, "klipy:wave");
        assert_eq!(asset.mime_type, "image/gif");
        assert!(asset.temporary);
        assert!(asset.file_path.exists());
        assert!(
            !asset
                .file_path
                .starts_with(database.media_storage_directory())
        );

        asset.cleanup();
        assert!(!asset.file_path.exists());

        handle.join().expect("test server should finish");
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn rejects_klipy_transient_fetches_with_unexpected_mime_type() {
        let (address, handle) = serve_test_media_response("image/png", b"PNG");
        let directory = temporary_path("dust-wave-social-klipy-transient-mime-test");
        fs::create_dir_all(&directory).expect("temporary directory should be created");

        let database_path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&database_path).expect("database should initialize");
        let media = ExternalMediaItem {
            id: "wave".to_string(),
            name: "Wave".to_string(),
            mime_type: "image/gif".to_string(),
            media_type: "gif".to_string(),
            url: format!("http://{address}/wave.gif"),
            thumb_url: format!("http://{address}/wave-thumb.gif"),
            is_video: false,
            credit_url: None,
            download_data: Some(serde_json::json!({ "provider": "klipy" })),
        };
        let error = database
            .transient_external_media_asset(&media)
            .expect_err("non-gif response should be rejected");

        assert!(
            error
                .to_string()
                .contains("external media response MIME type must match image/gif")
        );

        handle.join().expect("test server should finish");
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn enforces_mixpost_media_upload_type_and_size_limits() {
        validate_media_upload("image/png", MAX_IMAGE_BYTES).expect("max image should pass");
        validate_media_upload("image/gif", MAX_GIF_BYTES).expect("max gif should pass");
        validate_media_upload("video/mp4", MAX_VIDEO_BYTES).expect("max video should pass");

        assert_eq!(
            validate_media_upload("image/png", MAX_IMAGE_BYTES + 1)
                .expect_err("oversized image should fail")
                .to_string(),
            "image must not be greater than 5MB"
        );
        assert_eq!(
            validate_media_upload("image/gif", MAX_GIF_BYTES + 1)
                .expect_err("oversized gif should fail")
                .to_string(),
            "gif must not be greater than 15MB"
        );
        assert_eq!(
            validate_media_upload("image/webp", 1)
                .expect_err("unsupported mime should fail")
                .to_string(),
            "mime_type must be image/jpg, image/jpeg, image/gif, image/png, video/mp4, or video/x-m4v"
        );
    }

    #[test]
    fn imports_video_media_with_thumb_when_ffmpeg_is_available() {
        let ffmpeg = media_tool_path("FFMPEG_PATH", "ffmpeg");
        let ffprobe = media_tool_path("FFPROBE_PATH", "ffprobe");
        let ffmpeg_available = Command::new(&ffmpeg)
            .arg("-version")
            .output()
            .is_ok_and(|output| output.status.success());
        let ffprobe_available = Command::new(&ffprobe)
            .arg("-version")
            .output()
            .is_ok_and(|output| output.status.success());

        if !ffmpeg_available || !ffprobe_available {
            return;
        }

        let directory = std::env::temp_dir().join(format!(
            "dust-wave-social-video-import-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("temporary directory should be created");

        let source_path = directory.join("sample.mp4");
        let output = Command::new(&ffmpeg)
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=16x16:rate=1:duration=1",
                "-c:v",
                "mpeg4",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&source_path)
            .output()
            .expect("ffmpeg should run");

        if !output.status.success() {
            fs::remove_dir_all(directory).expect("temporary directory should be removed");
            return;
        }

        let database_path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&database_path).expect("database should initialize");
        let imported = database
            .import_media_file(&MediaImportForm {
                source_path: source_path.display().to_string(),
                name: None,
            })
            .expect("video media should import");

        assert_eq!(imported.mime_type, "video/mp4");
        assert_eq!(imported.conversion_count, 1);
        assert!(imported.size_total > imported.size);

        let connection = database.connection().expect("database should open");
        let conversions_json: String = connection
            .query_row(
                "SELECT conversions_json FROM media WHERE uuid = ?1",
                params![&imported.uuid],
                |row| row.get(0),
            )
            .expect("conversions should load");
        let conversions: serde_json::Value =
            serde_json::from_str(&conversions_json).expect("conversions should be json");
        let thumb_path = conversions[0]["path"]
            .as_str()
            .expect("thumb path should exist")
            .to_string();

        assert_eq!(conversions[0]["engine"], "VideoThumb");
        assert_eq!(conversions[0]["disk"], "local");
        assert_eq!(conversions[0]["name"], "thumb");
        assert!(directory.join(&thumb_path).exists());

        assert!(
            database
                .delete_media(&imported.uuid)
                .expect("video media should delete")
        );
        assert!(!directory.join(&imported.path).exists());
        assert!(!directory.join(&thumb_path).exists());

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn cleans_orphaned_media_files_without_touching_referenced_media() {
        let directory = std::env::temp_dir().join(format!(
            "dust-wave-social-media-cleanup-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("temporary directory should be created");

        let source_path = directory.join("sample.png");
        let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([21, 94, 82, 255]));
        image
            .save(&source_path)
            .expect("source media file should be written");

        let database_path = directory.join("dust.sqlite3");
        let database = Database::initialize_at(&database_path).expect("database should initialize");
        let imported = database
            .import_media_file(&MediaImportForm {
                source_path: source_path.display().to_string(),
                name: None,
            })
            .expect("media file should import");
        let retained_path = directory.join(&imported.path);
        let connection = database.connection().expect("database should open");
        let conversions_json: String = connection
            .query_row(
                "SELECT conversions_json FROM media WHERE uuid = ?1",
                params![&imported.uuid],
                |row| row.get(0),
            )
            .expect("conversions should load");
        let conversions: serde_json::Value =
            serde_json::from_str(&conversions_json).expect("conversions should be json");
        let retained_thumb_path = directory.join(
            conversions[0]["path"]
                .as_str()
                .expect("thumb path should exist"),
        );
        let orphan_path = directory.join("media/orphan.bin");
        fs::write(&orphan_path, [1, 2, 3, 4]).expect("orphan file should be written");

        let summary = database
            .cleanup_orphaned_media_files()
            .expect("cleanup should run");

        assert_eq!(summary.scanned, 3);
        assert_eq!(summary.retained, 2);
        assert_eq!(summary.deleted, 1);
        assert_eq!(summary.reclaimed_bytes, 4);
        assert!(retained_path.exists());
        assert!(retained_thumb_path.exists());
        assert!(!orphan_path.exists());

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn builds_account_reports_from_metrics_and_audience() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-report-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave X".to_string(),
                username: Some("dustwave".to_string()),
                provider: "twitter".to_string(),
                provider_id: "dw-x".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/twitter/dw-x".to_string(),
                data: None,
            })
            .expect("account should save");
        let connection = database.connection().expect("database should open");

        connection
            .execute_batch(
                "INSERT INTO metrics (account_id, data_json, date)
                 VALUES
                    (1, '{\"likes\":4,\"retweets\":2,\"impressions\":100}', '2026-06-21'),
                    (1, '{\"likes\":6,\"retweets\":3,\"impressions\":150}', '2026-06-22');

                 INSERT INTO audience (account_id, total, date)
                 VALUES
                    (1, 1000, '2026-06-21'),
                    (1, 1100, '2026-06-22');",
            )
            .expect("fixtures should insert");

        let report = database
            .account_report_for_end_date(
                account.id,
                "7_days",
                7,
                NaiveDate::from_ymd_opt(2026, 6, 23).expect("fixed date should be valid"),
            )
            .expect("report should build");

        assert_eq!(report.account_id, account.id);
        assert_eq!(report.provider, "twitter");
        assert_eq!(report.tier.as_deref(), Some("legacy"));
        assert_eq!(report.metrics[0].key, "likes");
        assert_eq!(report.metrics[0].value, 10);
        assert_eq!(report.metrics[1].value, 5);
        assert_eq!(report.metrics[2].value, 250);
        assert_eq!(report.audience.values.len(), 7);
        assert_eq!(report.audience.values[5], Some(1100));
        assert_eq!(report.audience.values[6], None);

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn builds_facebook_page_report_from_insights() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-facebook-report-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave Page".to_string(),
                username: None,
                provider: "facebook_page".to_string(),
                provider_id: "dw-page".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/facebook/dw-page".to_string(),
                data: None,
            })
            .expect("account should save");
        let connection = database.connection().expect("database should open");

        connection
            .execute_batch(
                "INSERT INTO facebook_insights (account_id, type, value, date, created_at, updated_at)
                 VALUES
                    (1, 2, 20, '2026-06-22', '2026-06-22T00:00:00Z', '2026-06-22T00:00:00Z'),
                    (1, 3, 500, '2026-06-22', '2026-06-22T00:00:00Z', '2026-06-22T00:00:00Z');",
            )
            .expect("fixtures should insert");

        let report = database
            .account_report_for_end_date(
                account.id,
                "7_days",
                7,
                NaiveDate::from_ymd_opt(2026, 6, 23).expect("fixed date should be valid"),
            )
            .expect("report should build");

        assert_eq!(report.provider, "facebook_page");
        assert_eq!(report.metrics[0].key, "page_post_engagements");
        assert_eq!(report.metrics[0].value, 20);
        assert_eq!(report.metrics[1].key, "page_posts_impressions");
        assert_eq!(report.metrics[1].value, 500);

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn imports_facebook_insight_rows() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-facebook-import-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave Page".to_string(),
                username: None,
                provider: "facebook_page".to_string(),
                provider_id: "dw-page".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/facebook/dw-page".to_string(),
                data: Some(serde_json::json!({ "auth": "facebook_user" })),
            })
            .expect("account should save");
        let mut connection = database.connection().expect("database should open");
        let transaction = connection.transaction().expect("transaction should start");
        let imported = import_facebook_insight_rows(
            &transaction,
            account.id,
            &[
                serde_json::json!({
                    "name": "page_post_engagements",
                    "values": [
                        { "value": 7, "end_time": "2026-06-22T07:00:00+0000" }
                    ]
                }),
                serde_json::json!({
                    "name": "page_posts_impressions",
                    "values": [
                        { "value": 70, "end_time": "2026-06-22T07:00:00+0000" }
                    ]
                }),
                serde_json::json!({
                    "name": "unsupported",
                    "values": [
                        { "value": 1, "end_time": "2026-06-22T07:00:00+0000" }
                    ]
                }),
            ],
        )
        .expect("insights should import");

        transaction.commit().expect("transaction should commit");

        assert_eq!(imported, 2);

        let count: i64 = database
            .connection()
            .expect("database should open")
            .query_row(
                "SELECT COUNT(*) FROM facebook_insights WHERE account_id = ?1",
                params![account.id],
                |row| row.get(0),
            )
            .expect("insight count should load");

        assert_eq!(count, 2);

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn ingests_reporting_rows_through_repositories() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-report-ingest-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let twitter = database
            .save_account(&AccountForm {
                name: "Dust Wave X".to_string(),
                username: Some("dustwave".to_string()),
                provider: "twitter".to_string(),
                provider_id: "dw-x".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/twitter/dw-x".to_string(),
                data: None,
            })
            .expect("twitter account should save");

        database
            .save_metric(&MetricForm {
                account_id: twitter.id,
                date: "2026-06-22".to_string(),
                data: serde_json::json!({ "likes": 1, "retweets": 2, "impressions": 3 }),
            })
            .expect("metric should save");
        database
            .save_metric(&MetricForm {
                account_id: twitter.id,
                date: "2026-06-22".to_string(),
                data: serde_json::json!({ "likes": 5, "retweets": 7, "impressions": 11 }),
            })
            .expect("metric should update");
        database
            .save_audience(&AudienceForm {
                account_id: twitter.id,
                date: "2026-06-22".to_string(),
                total: 1200,
            })
            .expect("audience should save");

        let twitter_report = database
            .account_report_for_end_date(
                twitter.id,
                "7_days",
                7,
                NaiveDate::from_ymd_opt(2026, 6, 23).expect("fixed date should be valid"),
            )
            .expect("twitter report should build");

        assert_eq!(twitter_report.metrics[0].value, 5);
        assert_eq!(twitter_report.metrics[1].value, 7);
        assert_eq!(twitter_report.metrics[2].value, 11);
        assert_eq!(twitter_report.audience.values[5], Some(1200));

        let facebook = database
            .save_account(&AccountForm {
                name: "Dust Wave Page".to_string(),
                username: None,
                provider: "facebook_page".to_string(),
                provider_id: "dw-page".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/facebook/dw-page".to_string(),
                data: None,
            })
            .expect("facebook account should save");

        database
            .save_facebook_insight(&FacebookInsightForm {
                account_id: facebook.id,
                insight_type: 2,
                date: "2026-06-22".to_string(),
                value: 20,
            })
            .expect("facebook insight should save");
        database
            .save_facebook_insight(&FacebookInsightForm {
                account_id: facebook.id,
                insight_type: 2,
                date: "2026-06-22".to_string(),
                value: 30,
            })
            .expect("facebook insight should update");

        let facebook_report = database
            .account_report_for_end_date(
                facebook.id,
                "7_days",
                7,
                NaiveDate::from_ymd_opt(2026, 6, 23).expect("fixed date should be valid"),
            )
            .expect("facebook report should build");

        assert_eq!(facebook_report.metrics[0].value, 30);

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn enqueues_reserves_and_finishes_jobs() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-job-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let due = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 1 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: None,
            })
            .expect("due job should enqueue");

        database
            .enqueue_job(&JobForm {
                kind: "import_metrics".to_string(),
                payload: serde_json::json!({ "account_id": 1 }),
                run_at: "2026-06-25T15:00:00Z".to_string(),
                idempotency_key: None,
            })
            .expect("future job should enqueue");

        assert_eq!(due.status, "pending");
        assert_eq!(due.attempts, 0);
        assert_eq!(pending_job_count(&database), 2);

        let reserved = database
            .reserve_due_jobs("2026-06-24T15:00:00Z", 10)
            .expect("due jobs should reserve");

        assert_eq!(reserved.len(), 1);
        assert_eq!(reserved[0].id, due.id);
        assert_eq!(reserved[0].status, "processing");
        assert_eq!(reserved[0].attempts, 1);

        let completed = database.complete_job(due.id).expect("job should complete");
        assert_eq!(completed.status, "completed");

        let future = database
            .reserve_due_jobs("2026-06-26T15:00:00Z", 10)
            .expect("future job should reserve");
        assert_eq!(future.len(), 1);

        let failed = database
            .fail_job(future[0].id, "provider rate limited")
            .expect("job should fail");
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.last_error.as_deref(), Some("provider rate limited"));

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn enqueues_jobs_idempotently_when_key_matches() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-job-idempotency-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let first = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 1 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: Some("publish_post:1".to_string()),
            })
            .expect("first job should enqueue");
        let second = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 1, "attempt": 2 }),
                run_at: "2026-06-24T16:00:00Z".to_string(),
                idempotency_key: Some(" publish_post:1 ".to_string()),
            })
            .expect("duplicate job should return existing row");
        let pending_count = count_matching_rows(
            &database.connection().expect("database should open"),
            "job_queue",
            "status = 'pending'",
        )
        .expect("pending jobs should count");

        assert_eq!(second.id, first.id);
        assert_eq!(second.run_at, "2026-06-24T16:00:00Z");
        assert_eq!(second.idempotency_key.as_deref(), Some("publish_post:1"));
        assert_eq!(pending_count, 1);

        let reserved = database
            .reserve_due_jobs("2026-06-24T16:00:00Z", 10)
            .expect("job should reserve");
        assert_eq!(reserved.len(), 1);
        assert_eq!(reserved[0].status, "processing");

        let in_flight = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 1, "attempt": 3 }),
                run_at: "2026-06-24T17:00:00Z".to_string(),
                idempotency_key: Some("publish_post:1".to_string()),
            })
            .expect("processing duplicate should return existing row");

        assert_eq!(in_flight.id, first.id);
        assert_eq!(in_flight.status, "processing");

        database
            .complete_job(first.id)
            .expect("first job should complete");
        let next = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 1 }),
                run_at: "2026-06-25T15:00:00Z".to_string(),
                idempotency_key: Some("publish_post:1".to_string()),
            })
            .expect("completed key should be reusable");

        assert_ne!(next.id, first.id);
        assert_eq!(next.idempotency_key.as_deref(), Some("publish_post:1"));

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn creates_local_draft_posts_with_versions_and_tags() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-post-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let tag = database
            .create_tag(&TagForm {
                name: "Drafts".to_string(),
                hex_color: "#2f80ed".to_string(),
            })
            .expect("tag should create");

        let post_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [],
            "tags": [tag.id],
            "scheduled_at": "2026-06-24T15:00:00Z",
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Local draft body",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");

        let post = database
            .create_draft_post(&post_form)
            .expect("draft post should create");

        assert_eq!(post.status, "draft");
        assert_eq!(post.schedule_status, "pending");
        assert_eq!(post.scheduled_at.as_deref(), Some("2026-06-24T15:00:00Z"));
        assert_eq!(post.tag_count, 1);
        assert_eq!(post.preview, "Local draft body");

        let snapshot = database
            .local_data_snapshot()
            .expect("snapshot should load");
        assert_eq!(snapshot.posts.len(), 1);

        assert!(
            database
                .delete_post(&post.uuid)
                .expect("post should soft delete")
        );

        let snapshot = database
            .local_data_snapshot()
            .expect("snapshot should load after delete");
        assert!(snapshot.posts.is_empty());

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn schedules_posts_and_enqueues_publish_jobs() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-schedule-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let connection = database.connection().expect("database should open");

        connection
            .execute(
                "INSERT INTO accounts (
                    id, uuid, name, username, provider, provider_id, data_json, authorized,
                    access_token_secret_ref, created_at, updated_at
                ) VALUES (
                    1, 'schedule-account-uuid', 'Dust Wave', 'dustwave', 'mastodon',
                    'dw-1', NULL, 1, 'secret://account/1',
                    '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z'
                )",
                [],
            )
            .expect("account should insert");

        let post_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [1],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Scheduled local draft",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");

        let post = database
            .create_draft_post(&post_form)
            .expect("draft should create");

        let scheduled = database
            .schedule_post(
                &post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect("post should schedule");

        assert_eq!(scheduled.status, "scheduled");
        assert_eq!(scheduled.account_count, 1);
        assert_eq!(
            scheduled.scheduled_at.as_deref(),
            Some("2026-06-24T15:00:00Z")
        );
        assert_eq!(pending_job_count(&database), 1);

        let reserved = database
            .reserve_due_jobs("2026-06-24T15:00:00Z", 10)
            .expect("publish job should reserve");
        assert_eq!(reserved.len(), 1);
        assert_eq!(reserved[0].kind, "publish_post");

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn bulk_deletes_posts_and_cancels_pending_publish_jobs() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-bulk-delete-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/1".to_string(),
                data: None,
            })
            .expect("account should save");
        let post_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [account.id],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Bulk delete scheduled draft",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let post = database
            .create_draft_post(&post_form)
            .expect("draft should create");

        database
            .schedule_post(
                &post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect("post should schedule");

        let summary = database
            .bulk_delete_posts(&BulkDeletePostsForm {
                uuids: vec![post.uuid.clone(), post.uuid.clone(), "missing".to_string()],
            })
            .expect("bulk delete should run");

        assert_eq!(summary.requested, 2);
        assert_eq!(summary.deleted, 1);
        assert_eq!(summary.cancelled_jobs, 1);

        let connection = database.connection().expect("database should open");
        let (deleted_at, job_status): (Option<String>, String) = connection
            .query_row(
                "SELECT p.deleted_at, jq.status
                 FROM posts p
                 INNER JOIN job_queue jq ON jq.payload_json = ?2
                 WHERE p.id = ?1",
                params![post.id, publish_job_payload(post.id)],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("post and job should load");

        assert!(deleted_at.is_some());
        assert_eq!(job_status, "cancelled");

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn imports_mastodon_status_rows_and_processes_metrics() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-mastodon-import-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/mastodon/dw-1".to_string(),
                data: Some(serde_json::json!({ "server": "mastodon.social" })),
            })
            .expect("account should save");
        let mut connection = database.connection().expect("database should open");
        let transaction = connection.transaction().expect("transaction should start");
        let imported = import_mastodon_status_rows(
            &transaction,
            account.id,
            &[
                serde_json::json!({
                    "id": "1",
                    "content": "Launch",
                    "created_at": "2026-06-23T12:00:00.000Z",
                    "replies_count": 1,
                    "reblogs_count": 2,
                    "favourites_count": 3
                }),
                serde_json::json!({
                    "id": "2",
                    "content": "Follow up",
                    "created_at": "2026-06-23T13:00:00.000Z",
                    "replies_count": 4,
                    "reblogs_count": 5,
                    "favourites_count": 6
                }),
            ],
        )
        .expect("statuses should import");
        let metric_days =
            process_mastodon_metric_days(&transaction, account.id).expect("metrics should process");

        transaction.commit().expect("transaction should commit");

        assert_eq!(imported, 2);
        assert_eq!(metric_days, 1);

        let data_json: String = database
            .connection()
            .expect("database should open")
            .query_row(
                "SELECT data_json FROM metrics WHERE account_id = ?1 AND date = '2026-06-23'",
                params![account.id],
                |row| row.get(0),
            )
            .expect("metric should load");

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&data_json).expect("metric should be json"),
            serde_json::json!({ "replies": 5, "reblogs": 7, "favourites": 9 })
        );

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn imports_twitter_post_rows_and_processes_metrics() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-twitter-import-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "twitter".to_string(),
                provider_id: "42".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/twitter/42".to_string(),
                data: Some(serde_json::json!({ "auth": "oauth2_pkce" })),
            })
            .expect("account should save");
        let mut connection = database.connection().expect("database should open");
        let transaction = connection.transaction().expect("transaction should start");
        let imported = import_twitter_post_rows(
            &transaction,
            account.id,
            &[
                serde_json::json!({
                    "id": "100",
                    "text": "Launch",
                    "created_at": "2026-06-23T12:00:00.000Z",
                    "public_metrics": {
                        "like_count": 1,
                        "reply_count": 2,
                        "retweet_count": 3,
                        "impression_count": 4,
                        "quote_count": 5,
                        "bookmark_count": 6
                    }
                }),
                serde_json::json!({
                    "id": "101",
                    "text": "Follow up",
                    "created_at": "2026-06-23T13:00:00.000Z",
                    "public_metrics": {
                        "like_count": 10,
                        "reply_count": 20,
                        "retweet_count": 30,
                        "impression_count": 40
                    }
                }),
            ],
        )
        .expect("posts should import");
        let metric_days =
            process_twitter_metric_days(&transaction, account.id).expect("metrics should process");

        transaction.commit().expect("transaction should commit");

        assert_eq!(imported, 2);
        assert_eq!(metric_days, 1);

        let data_json: String = database
            .connection()
            .expect("database should open")
            .query_row(
                "SELECT data_json FROM metrics WHERE account_id = ?1 AND date = '2026-06-23'",
                params![account.id],
                |row| row.get(0),
            )
            .expect("metric should load");

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&data_json).expect("metric should be json"),
            serde_json::json!({
                "likes": 11,
                "replies": 22,
                "retweets": 33,
                "impressions": 44
            })
        );

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn enqueues_account_import_jobs_idempotently() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-account-import-job-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/mastodon/dw-1".to_string(),
                data: None,
            })
            .expect("account should save");
        let first = database
            .enqueue_account_import(&account.uuid, "2026-06-24T15:00:00Z")
            .expect("import job should enqueue");
        let second = database
            .enqueue_account_import(&account.uuid, "2026-06-24T16:00:00Z")
            .expect("duplicate import job should return existing row");

        assert_eq!(first.id, second.id);
        assert_eq!(second.kind, "import_account_data");
        assert_eq!(second.run_at, "2026-06-24T16:00:00Z");
        assert_eq!(
            second.idempotency_key,
            Some(format!("import_account:{}", account.id))
        );

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn account_import_jobs_survive_database_reopen() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-account-import-reopen-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let (account_uuid, first_job_id) = {
            let database = Database::initialize_at(&path).expect("database should initialize");
            let account = database
                .save_account(&AccountForm {
                    name: "Dust Wave".to_string(),
                    username: Some("dustwave".to_string()),
                    provider: "mastodon".to_string(),
                    provider_id: "dw-reopen".to_string(),
                    authorized: true,
                    avatar_path: None,
                    access_token_secret_ref: "secret://accounts/mastodon/dw-reopen".to_string(),
                    data: None,
                })
                .expect("account should save");
            let job = database
                .enqueue_account_import(&account.uuid, "2026-06-24T15:00:00Z")
                .expect("import job should enqueue");

            (account.uuid, job.id)
        };

        let database = Database::initialize_at(&path).expect("database should reopen");
        let second = database
            .enqueue_account_import(&account_uuid, "2026-06-24T16:00:00Z")
            .expect("existing import job should be reused after reopen");

        assert_eq!(second.id, first_job_id);
        assert_eq!(second.run_at, "2026-06-24T16:00:00Z");

        let connection = database.connection().expect("database should open");
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM job_queue
                 WHERE kind = 'import_account_data'
                   AND idempotency_key = ?1",
                params![second.idempotency_key.as_deref().unwrap()],
                |row| row.get(0),
            )
            .expect("import job count should load");

        assert_eq!(count, 1);

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn queues_import_jobs_for_all_connected_supported_accounts() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-account-import-batch-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let authorized = database
            .save_account(&AccountForm {
                name: "Dust Wave Mastodon".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-masto".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/mastodon/dw-masto".to_string(),
                data: None,
            })
            .expect("authorized account should save");
        database
            .save_account(&AccountForm {
                name: "Dust Wave X".to_string(),
                username: Some("dustwave".to_string()),
                provider: "twitter".to_string(),
                provider_id: "dw-x".to_string(),
                authorized: false,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/twitter/dw-x".to_string(),
                data: None,
            })
            .expect("unauthorized account should save");
        database
            .save_account(&AccountForm {
                name: "Dust Wave Unsupported".to_string(),
                username: None,
                provider: "linkedin".to_string(),
                provider_id: "dw-linkedin".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://accounts/linkedin/dw-linkedin".to_string(),
                data: None,
            })
            .expect("unsupported account should save");

        let summary = database
            .enqueue_all_account_imports("2026-06-24T15:00:00Z")
            .expect("batch import jobs should enqueue");

        assert_eq!(summary.requested_accounts, 3);
        assert_eq!(summary.eligible_accounts, 1);
        assert_eq!(summary.queued_jobs, 1);
        assert_eq!(summary.skipped_unauthorized, 1);
        assert_eq!(summary.skipped_unsupported, 1);
        assert_eq!(
            summary.jobs[0].idempotency_key,
            Some(format!("import_account:{}", authorized.id))
        );

        let second = database
            .enqueue_all_account_imports("2026-06-24T16:00:00Z")
            .expect("batch import jobs should remain idempotent");

        assert_eq!(second.queued_jobs, 1);
        assert_eq!(second.jobs[0].id, summary.jobs[0].id);
        assert_eq!(second.jobs[0].run_at, "2026-06-24T16:00:00Z");

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn worker_fails_account_import_jobs_for_missing_accounts() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-missing-account-import-job-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        database
            .enqueue_job(&JobForm {
                kind: "import_account_data".to_string(),
                payload: serde_json::json!({ "account_id": 999 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: Some("import_account:999".to_string()),
            })
            .expect("import job should enqueue");

        let summary = database
            .run_due_jobs("2026-06-24T15:00:00Z", 10)
            .expect("worker should run");

        assert_eq!(summary.reserved, 1);
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.failed, 1);
        assert!(summary.outcomes[0].detail.contains("account 999 not found"));

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn retries_only_failed_account_import_jobs() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-retry-account-import-job-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let import_job = database
            .enqueue_job(&JobForm {
                kind: "import_account_data".to_string(),
                payload: serde_json::json!({ "account_id": 1 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: Some("import_account:1".to_string()),
            })
            .expect("import job should enqueue");
        let publish_job = database
            .enqueue_job(&JobForm {
                kind: "publish_post".to_string(),
                payload: serde_json::json!({ "post_id": 1 }),
                run_at: "2026-06-24T15:00:00Z".to_string(),
                idempotency_key: Some("publish_post:1".to_string()),
            })
            .expect("publish job should enqueue");

        database
            .fail_job(import_job.id, "provider timeout")
            .expect("import job should fail");
        database
            .fail_job(publish_job.id, "provider timeout")
            .expect("publish job should fail");

        let retried = database
            .retry_failed_account_import_jobs("2026-06-24T16:00:00Z")
            .expect("failed account imports should retry");

        assert_eq!(retried.len(), 1);
        assert_eq!(retried[0].id, import_job.id);
        assert_eq!(retried[0].status, "pending");
        assert_eq!(retried[0].run_at, "2026-06-24T16:00:00Z");
        assert!(retried[0].last_error.is_none());

        let connection = database.connection().expect("database should open");
        let publish_status: String = connection
            .query_row(
                "SELECT status FROM job_queue WHERE id = ?1",
                params![publish_job.id],
                |row| row.get(0),
            )
            .expect("publish job should load");

        assert_eq!(publish_status, "failed");

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn validates_provider_limits_before_scheduling() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-validation-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave X".to_string(),
                username: Some("dustwave".to_string()),
                provider: "twitter".to_string(),
                provider_id: "dw-x".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/twitter".to_string(),
                data: None,
            })
            .expect("account should save");
        let long_body = "x".repeat(281);
        let post_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [account.id],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": long_body,
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let post = database
            .create_draft_post(&post_form)
            .expect("post should create");
        let report = database
            .validate_post(&post.uuid)
            .expect("post should validate");

        assert!(!report.valid);
        assert_eq!(report.errors[0].code, "text_too_long");

        let error = database
            .schedule_post(
                &post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect_err("invalid post should not schedule");

        assert!(error.to_string().contains("281 characters"));
        assert_eq!(pending_job_count(&database), 0);

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn validates_and_summarizes_klipy_external_media_references() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-klipy-reference-validation-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave Mastodon".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-masto".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/mastodon".to_string(),
                data: Some(serde_json::json!({ "server": "mastodon.social" })),
            })
            .expect("account should save");
        let post_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [account.id],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "",
                            "media": [],
                            "external_media": [
                                {
                                    "id": "wave",
                                    "name": "Wave",
                                    "mime_type": "image/gif",
                                    "media_type": "gif",
                                    "url": "https://cdn.klipy.com/wave.gif",
                                    "thumb_url": "https://cdn.klipy.com/wave-thumb.gif",
                                    "is_video": false,
                                    "credit_url": null,
                                    "download_data": { "provider": "klipy" }
                                }
                            ]
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let post = database
            .create_draft_post(&post_form)
            .expect("post should create");
        let report = database
            .validate_post(&post.uuid)
            .expect("post should validate");
        let posts = database
            .query_posts(&PostQueryRequest {
                status: Some("draft".to_string()),
                exclude_status: None,
                keyword: None,
                accounts: vec![],
                tags: vec![],
                calendar_type: None,
                date: None,
                limit: Some(20),
                page: None,
            })
            .expect("post query should run");

        assert!(report.valid);
        assert_eq!(database.media().expect("media should load").len(), 0);
        assert_eq!(posts.items[0].external_media.len(), 1);
        assert_eq!(posts.items[0].external_media[0].id, "wave");

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn validates_simultaneous_posting_provider_rules() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-simultaneous-posting-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let first = database
            .save_account(&AccountForm {
                name: "Dust Wave X".to_string(),
                username: Some("dustwave".to_string()),
                provider: "twitter".to_string(),
                provider_id: "dw-x".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/twitter/one".to_string(),
                data: None,
            })
            .expect("first account should save");
        let second = database
            .save_account(&AccountForm {
                name: "Dust Wave Support".to_string(),
                username: Some("dustwavesupport".to_string()),
                provider: "twitter".to_string(),
                provider_id: "dw-x-support".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/twitter/two".to_string(),
                data: None,
            })
            .expect("second account should save");
        let post_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [first.id, second.id],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Two X accounts should not publish together",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let post = database
            .create_draft_post(&post_form)
            .expect("post should create");
        let report = database
            .validate_post(&post.uuid)
            .expect("post should validate");

        assert!(!report.valid);
        assert!(report.errors.iter().any(|error| {
            error.provider == "twitter" && error.code == "simultaneous_posting_disabled"
        }));

        let error = database
            .schedule_post(
                &post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect_err("invalid simultaneous post should not schedule");

        assert!(error.to_string().contains("simultaneous posting"));

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn worker_fails_scheduled_posts_that_become_invalid() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-worker-validation-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-masto".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/mastodon".to_string(),
                data: None,
            })
            .expect("account should save");
        let post_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [account.id],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Valid body",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let post = database
            .create_draft_post(&post_form)
            .expect("post should create");

        database
            .schedule_post(
                &post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect("post should schedule");

        database
            .connection()
            .expect("database should open")
            .execute(
                "UPDATE post_versions
                 SET content_json = ?2
                 WHERE post_id = ?1",
                params![
                    post.id,
                    serde_json::json!([
                        {
                            "body": "x".repeat(501),
                            "media": []
                        }
                    ])
                    .to_string()
                ],
            )
            .expect("fixture should mutate post content");

        let summary = database
            .run_due_jobs("2026-06-24T15:00:00Z", 10)
            .expect("worker should run");

        assert_eq!(summary.failed, 1);
        assert!(summary.outcomes[0].detail.contains("501 characters"));

        let connection = database.connection().expect("database should open");
        let failed = database
            .post_by_id(&connection, post.id)
            .expect("post should load");
        assert_eq!(failed.status, "failed");

        let errors_json: String = connection
            .query_row(
                "SELECT errors_json
                 FROM post_accounts
                 WHERE post_id = ?1 AND account_id = ?2",
                params![post.id, account.id],
                |row| row.get(0),
            )
            .expect("post account errors should load");
        assert!(errors_json.contains("501 characters"));

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn updates_posts_and_reconciles_pending_publish_jobs() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-update-post-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let mastodon = database
            .save_account(&AccountForm {
                name: "Dust Wave Mastodon".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-masto".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/mastodon".to_string(),
                data: None,
            })
            .expect("mastodon account should save");
        let twitter = database
            .save_account(&AccountForm {
                name: "Dust Wave X".to_string(),
                username: Some("dustwave".to_string()),
                provider: "twitter".to_string(),
                provider_id: "dw-x".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/twitter".to_string(),
                data: None,
            })
            .expect("twitter account should save");
        let launch = database
            .create_tag(&TagForm {
                name: "Launch".to_string(),
                hex_color: "#2f80ed".to_string(),
            })
            .expect("launch tag should create");
        let campaign = database
            .create_tag(&TagForm {
                name: "Campaign".to_string(),
                hex_color: "#22ff88".to_string(),
            })
            .expect("campaign tag should create");
        let post_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [mastodon.id],
            "tags": [launch.id],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Original body",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let post = database
            .create_draft_post(&post_form)
            .expect("post should create");

        database
            .schedule_post(
                &post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect("post should schedule");

        let edited_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [twitter.id],
            "tags": [campaign.id],
            "scheduled_at": "2026-06-25T16:30:00Z",
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Edited body",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("edited form should deserialize");
        let edited = database
            .update_post(&post.uuid, &edited_form)
            .expect("post should update");

        assert_eq!(edited.status, "scheduled");
        assert_eq!(edited.scheduled_at.as_deref(), Some("2026-06-25T16:30:00Z"));
        assert_eq!(edited.account_count, 1);
        assert_eq!(edited.tag_count, 1);
        assert_eq!(edited.preview, "Edited body");

        let detail = database
            .post_detail(&post.uuid)
            .expect("post detail should load");
        assert_eq!(detail.accounts, vec![twitter.id]);
        assert_eq!(detail.tags, vec![campaign.id]);
        assert_eq!(detail.versions[0].content[0].body, "Edited body");

        let connection = database.connection().expect("database should open");
        let run_at: String = connection
            .query_row(
                "SELECT run_at
                 FROM job_queue
                 WHERE kind = 'publish_post' AND payload_json = ?1",
                params![publish_job_payload(post.id)],
                |row| row.get(0),
            )
            .expect("publish job should load");
        assert_eq!(run_at, "2026-06-25T16:30:00Z");

        let draft_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Back to draft",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("draft form should deserialize");
        let draft = database
            .update_post(&post.uuid, &draft_form)
            .expect("post should return to draft");

        assert_eq!(draft.status, "draft");
        assert_eq!(draft.account_count, 0);
        assert_eq!(pending_job_count(&database), 0);

        let job_status: String = database
            .connection()
            .expect("database should open")
            .query_row(
                "SELECT status
                 FROM job_queue
                 WHERE kind = 'publish_post' AND payload_json = ?1",
                params![publish_job_payload(post.id)],
                |row| row.get(0),
            )
            .expect("publish job should load");
        assert_eq!(job_status, "cancelled");

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn duplicates_posts_as_drafts_with_relationships_and_versions() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-duplicate-post-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/1".to_string(),
                data: None,
            })
            .expect("account should save");
        let tag = database
            .create_tag(&TagForm {
                name: "Launch".to_string(),
                hex_color: "#2f80ed".to_string(),
            })
            .expect("tag should create");
        let form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [account.id],
            "tags": [tag.id],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Duplicate source",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let source = database
            .create_draft_post(&form)
            .expect("source post should create");

        database
            .schedule_post(
                &source.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect("source post should schedule");

        let duplicate = database
            .duplicate_post(&source.uuid)
            .expect("post should duplicate");

        assert_ne!(duplicate.uuid, source.uuid);
        assert_eq!(duplicate.status, "draft");
        assert_eq!(duplicate.schedule_status, "pending");
        assert!(duplicate.scheduled_at.is_none());
        assert_eq!(duplicate.account_count, 1);
        assert_eq!(duplicate.tag_count, 1);
        assert_eq!(duplicate.preview, "Duplicate source");

        let detail = database
            .post_detail(&duplicate.uuid)
            .expect("duplicate detail should load");
        assert_eq!(detail.accounts, vec![account.id]);
        assert_eq!(detail.tags, vec![tag.id]);
        assert_eq!(detail.versions[0].content[0].body, "Duplicate source");
        assert_eq!(pending_job_count(&database), 1);

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn queries_posts_with_status_keyword_account_and_tag_filters() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-post-query-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let mastodon = database
            .save_account(&AccountForm {
                name: "Dust Wave Mastodon".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-masto".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/mastodon".to_string(),
                data: None,
            })
            .expect("mastodon account should save");
        let twitter = database
            .save_account(&AccountForm {
                name: "Dust Wave X".to_string(),
                username: Some("dustwave".to_string()),
                provider: "twitter".to_string(),
                provider_id: "dw-x".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/twitter".to_string(),
                data: None,
            })
            .expect("twitter account should save");
        let launch = database
            .create_tag(&TagForm {
                name: "Launch".to_string(),
                hex_color: "#2f80ed".to_string(),
            })
            .expect("tag should create");

        let launch_post = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [mastodon.id],
            "tags": [launch.id],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Launch plan for Dust Wave",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let launch_post = database
            .create_draft_post(&launch_post)
            .expect("launch post should create");
        database
            .schedule_post(
                &launch_post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect("launch post should schedule");

        let other_post = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [twitter.id],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Unrelated update",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let other_post = database
            .create_draft_post(&other_post)
            .expect("other post should create");
        database
            .schedule_post(
                &other_post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-25T15:00:00Z".to_string(),
                },
            )
            .expect("other post should schedule");

        let result = database
            .query_posts(&PostQueryRequest {
                status: Some("scheduled".to_string()),
                exclude_status: None,
                keyword: Some("launch".to_string()),
                accounts: vec![mastodon.id],
                tags: vec![launch.id],
                calendar_type: None,
                date: None,
                limit: Some(20),
                page: None,
            })
            .expect("post query should run");

        assert_eq!(result.total, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].uuid, launch_post.uuid);
        assert_eq!(result.items[0].preview, "Launch plan for Dust Wave");
        assert!(!result.has_failed_posts);

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn queries_calendar_windows_and_excludes_drafts() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-calendar-query-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/1".to_string(),
                data: None,
            })
            .expect("account should save");

        for (body, scheduled_at) in [
            ("Week post", "2026-06-24T15:00:00Z"),
            ("Next week post", "2026-06-29T15:00:00Z"),
        ] {
            let form = serde_json::from_value::<PostForm>(serde_json::json!({
                "accounts": [account.id],
                "tags": [],
                "scheduled_at": null,
                "versions": [
                    {
                        "account_id": 0,
                        "is_original": true,
                        "content": [
                            {
                                "body": body,
                                "media": []
                            }
                        ]
                    }
                ]
            }))
            .expect("post form should deserialize");
            let post = database
                .create_draft_post(&form)
                .expect("post should create");
            database
                .schedule_post(
                    &post.uuid,
                    &SchedulePostForm {
                        scheduled_at: scheduled_at.to_string(),
                    },
                )
                .expect("post should schedule");
        }

        let draft = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [],
            "tags": [],
            "scheduled_at": "2026-06-24T18:00:00Z",
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Draft in same week",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("draft form should deserialize");
        database
            .create_draft_post(&draft)
            .expect("draft should create");

        let result = database
            .query_posts(&PostQueryRequest {
                status: None,
                exclude_status: Some("draft".to_string()),
                keyword: None,
                accounts: vec![],
                tags: vec![],
                calendar_type: Some("week".to_string()),
                date: Some("2026-06-24".to_string()),
                limit: Some(20),
                page: None,
            })
            .expect("calendar query should run");
        let window = result
            .calendar_window
            .expect("calendar window should be present");

        assert_eq!(window.start_date, "2026-06-22");
        assert_eq!(window.end_date, "2026-06-28");
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].preview, "Week post");

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn worker_fails_scheduled_posts_missing_provider_connection() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-worker-publish-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/1".to_string(),
                data: None,
            })
            .expect("account should save");
        let post_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [account.id],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Worker publish draft",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let post = database
            .create_draft_post(&post_form)
            .expect("draft should create");

        database
            .schedule_post(
                &post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect("post should schedule");

        let summary = database
            .run_due_jobs("2026-06-24T15:00:00Z", 10)
            .expect("worker should run");

        assert_eq!(summary.reserved, 1);
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.outcomes[0].status, "failed");
        assert!(summary.outcomes[0].detail.contains("must be connected"));

        let failed = database
            .post_by_id(
                &database.connection().expect("database should open"),
                post.id,
            )
            .expect("post should load");
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.schedule_status, "processed");
        assert_eq!(failed.published_at.as_deref(), None);
        assert_eq!(pending_job_count(&database), 0);

        let connection = database.connection().expect("database should open");
        let (provider_post_id, errors_json): (Option<String>, Option<String>) = connection
            .query_row(
                "SELECT provider_post_id, errors_json
                 FROM post_accounts
                 WHERE post_id = ?1 AND account_id = ?2",
                params![post.id, account.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("post account should load");
        assert!(provider_post_id.is_none());
        assert!(
            errors_json
                .as_deref()
                .is_some_and(|value| value.contains("must be connected"))
        );

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn worker_marks_unauthorized_scheduled_posts_failed() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-worker-fail-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: false,
                avatar_path: None,
                access_token_secret_ref: "secret://account/1".to_string(),
                data: None,
            })
            .expect("account should save");
        let post_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [account.id],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Worker fail draft",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let post = database
            .create_draft_post(&post_form)
            .expect("draft should create");

        database
            .schedule_post(
                &post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect("post should schedule");

        let summary = database
            .run_due_jobs("2026-06-24T15:00:00Z", 10)
            .expect("worker should run");

        assert_eq!(summary.reserved, 1);
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.failed, 1);
        assert!(summary.outcomes[0].detail.contains("need authorization"));

        let failed = database
            .post_by_id(
                &database.connection().expect("database should open"),
                post.id,
            )
            .expect("post should load");
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.schedule_status, "processed");

        let connection = database.connection().expect("database should open");
        let (job_status, last_error, errors_json): (String, Option<String>, Option<String>) =
            connection
                .query_row(
                    "SELECT jq.status, jq.last_error, pa.errors_json
                     FROM job_queue jq
                     INNER JOIN post_accounts pa ON pa.post_id = ?2
                     WHERE jq.id = ?1",
                    params![summary.outcomes[0].job_id, post.id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("job and post account should load");

        assert_eq!(job_status, "failed");
        assert!(
            last_error
                .unwrap_or_default()
                .contains("need authorization")
        );
        assert_eq!(errors_json.as_deref(), Some(r#"["Access token expired"]"#));

        connection
            .execute(
                "UPDATE accounts SET authorized = 1 WHERE id = ?1",
                params![account.id],
            )
            .expect("account should reauthorize");

        let retried = database
            .retry_failed_post(
                &post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T16:00:00Z".to_string(),
                },
            )
            .expect("failed post should retry");

        assert_eq!(retried.status, "scheduled");
        assert_eq!(retried.schedule_status, "pending");
        assert_eq!(
            retried.scheduled_at.as_deref(),
            Some("2026-06-24T16:00:00Z")
        );

        let (pending_jobs, retry_errors_json): (i64, Option<String>) = connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM job_queue WHERE status = 'pending' AND payload_json = ?1),
                    (SELECT errors_json FROM post_accounts WHERE post_id = ?2)
                 ",
                params![publish_job_payload(post.id), post.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("retry state should load");

        assert_eq!(pending_jobs, 1);
        assert!(retry_errors_json.is_none());

        fs::remove_file(path).expect("temporary database should be removed");
    }

    #[test]
    fn worker_defers_publish_jobs_when_rate_limited() {
        let path = std::env::temp_dir().join(format!(
            "dust-wave-social-rate-limit-worker-test-{}.sqlite3",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let database = Database::initialize_at(&path).expect("database should initialize");
        let account = database
            .save_account(&AccountForm {
                name: "Dust Wave".to_string(),
                username: Some("dustwave".to_string()),
                provider: "mastodon".to_string(),
                provider_id: "dw-1".to_string(),
                authorized: true,
                avatar_path: None,
                access_token_secret_ref: "secret://account/1".to_string(),
                data: None,
            })
            .expect("account should save");
        let post_form = serde_json::from_value::<PostForm>(serde_json::json!({
            "accounts": [account.id],
            "tags": [],
            "scheduled_at": null,
            "versions": [
                {
                    "account_id": 0,
                    "is_original": true,
                    "content": [
                        {
                            "body": "Rate limited draft",
                            "media": []
                        }
                    ]
                }
            ]
        }))
        .expect("post form should deserialize");
        let post = database
            .create_draft_post(&post_form)
            .expect("draft should create");

        database
            .schedule_post(
                &post.uuid,
                &SchedulePostForm {
                    scheduled_at: "2026-06-24T15:00:00Z".to_string(),
                },
            )
            .expect("post should schedule");

        let rate_limit = database
            .save_rate_limit(&RateLimitForm {
                scope: account_rate_limit_scope(account.id),
                retry_after_at: "2026-06-24T16:00:00Z".to_string(),
                payload: Some(serde_json::json!({ "source": "test" })),
            })
            .expect("rate limit should save");

        assert_eq!(rate_limit.scope, account_rate_limit_scope(account.id));
        assert_eq!(database.rate_limits().expect("limits should load").len(), 1);
        assert_eq!(
            database
                .local_data_snapshot()
                .expect("snapshot should load")
                .rate_limits
                .len(),
            1
        );

        let deferred = database
            .run_due_jobs("2026-06-24T15:00:00Z", 10)
            .expect("worker should run");

        assert_eq!(deferred.reserved, 1);
        assert_eq!(deferred.deferred, 1);
        assert_eq!(deferred.completed, 0);
        assert_eq!(deferred.failed, 0);
        assert_eq!(deferred.outcomes[0].status, "deferred");

        let connection = database.connection().expect("database should open");
        let (job_status, run_at, last_error): (String, String, Option<String>) = connection
            .query_row(
                "SELECT status, run_at, last_error
                 FROM job_queue
                 WHERE kind = 'publish_post' AND payload_json = ?1",
                params![publish_job_payload(post.id)],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("job should load");

        assert_eq!(job_status, "pending");
        assert_eq!(run_at, "2026-06-24T16:00:00Z");
        assert!(
            last_error
                .unwrap_or_default()
                .contains("deferred by rate limit")
        );

        let scheduled = database
            .post_by_id(&connection, post.id)
            .expect("post should load");
        assert_eq!(scheduled.status, "scheduled");
        assert_eq!(scheduled.schedule_status, "pending");

        assert!(
            database
                .clear_rate_limit(&account_rate_limit_scope(account.id))
                .expect("rate limit should clear")
        );
        assert!(
            database
                .rate_limits()
                .expect("limits should load")
                .is_empty()
        );

        let failed_after_limit = database
            .run_due_jobs("2026-06-24T16:00:00Z", 10)
            .expect("worker should retry after limit clears");

        assert_eq!(failed_after_limit.completed, 0);
        assert_eq!(failed_after_limit.deferred, 0);
        assert_eq!(failed_after_limit.failed, 1);
        assert!(
            failed_after_limit.outcomes[0]
                .detail
                .contains("must be connected")
        );

        fs::remove_file(path).expect("temporary database should be removed");
    }
}
