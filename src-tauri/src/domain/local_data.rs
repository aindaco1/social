use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct LocalDataSnapshot {
    pub accounts: Vec<AccountSummary>,
    pub services: Vec<ServiceSummary>,
    pub posts: Vec<PostSummary>,
    pub media: Vec<MediaSummary>,
    pub tags: Vec<TagSummary>,
    pub jobs: Vec<JobSummary>,
    pub rate_limits: Vec<RateLimitSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalBackupExportSummary {
    pub path: String,
    pub database_path: String,
    pub media_path: String,
    pub manifest_path: String,
    pub media_files: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalBackupRestoreForm {
    pub backup_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalBackupRestoreSummary {
    pub backup_path: String,
    pub safety_backup_path: String,
    pub database_path: String,
    pub media_path: String,
    pub restored_media_files: usize,
    pub restored_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ValidatedLocalBackupRestore {
    pub backup_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemLogFile {
    pub name: String,
    pub contents: String,
    pub error: Option<String>,
    pub bytes: u64,
    pub entry_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemLogExportSummary {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemLogClearSummary {
    pub deleted_entries: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemHealthCounts {
    pub unauthorized_accounts: i64,
    pub failed_posts: i64,
    pub pending_jobs: i64,
    pub processing_jobs: i64,
    pub failed_jobs: i64,
    pub rate_limits: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemHealthIssue {
    pub severity: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemMediaToolStatus {
    pub name: String,
    pub command: String,
    pub available: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemMediaToolSummary {
    pub ffmpeg: SystemMediaToolStatus,
    pub ffprobe: SystemMediaToolStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemHealthSummary {
    pub generated_at: String,
    pub status: String,
    pub counts: SystemHealthCounts,
    pub issues: Vec<SystemHealthIssue>,
    pub media_tools: SystemMediaToolSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemMaintenanceSummary {
    pub completed_jobs_deleted: usize,
    pub cancelled_jobs_deleted: usize,
    pub expired_rate_limits_cleared: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DesktopMaintenanceSummary {
    pub now: String,
    pub resolved_state: SystemMaintenanceSummary,
    pub media: MediaCleanupSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaleJobRecoverySummary {
    pub requeued_jobs: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSummary {
    pub generated_at: String,
    pub accounts: DashboardAccountCounts,
    pub posts: DashboardPostCounts,
    pub jobs: DashboardJobCounts,
    pub providers: Vec<DashboardProviderSummary>,
    pub upcoming_posts: Vec<PostSummary>,
    pub failed_posts: Vec<PostSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardAccountCounts {
    pub total: i64,
    pub authorized: i64,
    pub unauthorized: i64,
    pub providers: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardPostCounts {
    pub draft: i64,
    pub scheduled: i64,
    pub publishing: i64,
    pub published: i64,
    pub failed: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardJobCounts {
    pub pending: i64,
    pub processing: i64,
    pub failed: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardProviderSummary {
    pub provider: String,
    pub accounts: i64,
    pub authorized_accounts: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MastodonAppRegistrationForm {
    pub server: String,
    pub client_name: Option<String>,
    pub redirect_uri: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidatedMastodonAppRegistration {
    pub server: String,
    pub client_name: String,
    pub redirect_uri: String,
    pub website: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MastodonAppRegistrationSummary {
    pub server: String,
    pub service_name: String,
    pub client_id_ref: String,
    pub client_secret_ref: String,
    pub auth_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MastodonOAuthForm {
    pub server: String,
    pub code: String,
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidatedMastodonOAuth {
    pub server: String,
    pub code: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MastodonAccountConnection {
    pub server: String,
    pub account: AccountSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct MastodonImportSummary {
    pub account: AccountSummary,
    pub audience_total: Option<i64>,
    pub imported_posts: usize,
    pub metric_days: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TwitterOAuthStartForm {
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TwitterOAuthStartSummary {
    pub auth_url: String,
    pub code_verifier: String,
    pub state: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TwitterOAuthExchangeForm {
    pub code: String,
    pub code_verifier: String,
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TwitterAccountConnection {
    pub account: AccountSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct TwitterImportSummary {
    pub account: AccountSummary,
    pub audience_total: Option<i64>,
    pub imported_posts: usize,
    pub metric_days: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FacebookOAuthStartForm {
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub api_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FacebookOAuthStartSummary {
    pub auth_url: String,
    pub state: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub api_version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FacebookOAuthExchangeForm {
    pub code: String,
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub api_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FacebookPageCandidate {
    pub id: String,
    pub name: String,
    pub username: Option<String>,
    pub avatar_path: Option<String>,
    #[serde(skip_serializing)]
    pub access_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FacebookUserConnectionSummary {
    pub user_id: String,
    pub user_name: String,
    pub pages: Vec<FacebookPageCandidate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FacebookPageConnectForm {
    pub user_id: String,
    pub page_ids: Vec<String>,
    #[serde(default)]
    pub api_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FacebookPageConnectionSummary {
    pub accounts: Vec<AccountSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FacebookImportSummary {
    pub account: AccountSummary,
    pub audience_total: Option<i64>,
    pub insight_rows: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountSummary {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub username: Option<String>,
    pub provider: String,
    pub provider_id: String,
    pub authorized: bool,
    pub avatar_path: Option<String>,
    pub access_token_secret_ref: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountForm {
    pub name: String,
    pub username: Option<String>,
    pub provider: String,
    pub provider_id: String,
    pub authorized: bool,
    pub avatar_path: Option<String>,
    pub access_token_secret_ref: String,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ValidatedAccount {
    pub name: String,
    pub username: Option<String>,
    pub provider: String,
    pub provider_id: String,
    pub authorized: bool,
    pub avatar_path: Option<String>,
    pub access_token_secret_ref: String,
    pub data_json: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceSummary {
    pub id: i64,
    pub name: String,
    pub configuration_secret_ref: String,
    pub configuration: Value,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceCredentialStatus {
    pub service: String,
    pub label: String,
    pub group: String,
    pub active: bool,
    pub configured: bool,
    pub fields: Vec<ServiceCredentialFieldStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceCredentialFieldStatus {
    pub field: String,
    pub label: String,
    pub configured: bool,
    pub env_vars: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceCredentialForm {
    pub service: String,
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceForm {
    pub name: String,
    pub configuration_secret_ref: String,
    #[serde(default)]
    pub configuration: Option<Value>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct ValidatedService {
    pub name: String,
    pub configuration_secret_ref: String,
    pub configuration_json: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct ValidatedServiceCredential {
    pub service: String,
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostListAccount {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub username: Option<String>,
    pub provider: String,
    pub avatar_path: Option<String>,
    pub authorized: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostListTag {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub hex_color: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostSummary {
    pub id: i64,
    pub uuid: String,
    pub status: String,
    pub schedule_status: String,
    pub scheduled_at: Option<String>,
    pub published_at: Option<String>,
    pub account_count: i64,
    pub tag_count: i64,
    pub accounts: Vec<PostListAccount>,
    pub tags: Vec<PostListTag>,
    pub media: Vec<MediaLibraryItem>,
    pub external_media: Vec<ExternalMediaItem>,
    pub failure_errors: Vec<String>,
    pub preview: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing)]
    pub media_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BulkDeletePostsForm {
    pub uuids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkDeletePostsSummary {
    pub requested: usize,
    pub deleted: usize,
    pub cancelled_jobs: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostQueryResult {
    pub items: Vec<PostSummary>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
    pub has_failed_posts: bool,
    pub calendar_window: Option<PostCalendarWindow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostDetail {
    pub id: i64,
    pub uuid: String,
    pub status: String,
    pub schedule_status: String,
    pub scheduled_at: Option<String>,
    pub published_at: Option<String>,
    pub accounts: Vec<i64>,
    pub tags: Vec<i64>,
    pub versions: Vec<PostVersionForm>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostValidationReport {
    pub valid: bool,
    pub errors: Vec<PostValidationError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostValidationError {
    pub account_id: i64,
    pub provider: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostCalendarWindow {
    pub calendar_type: String,
    pub selected_date: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostQueryRequest {
    pub status: Option<String>,
    pub exclude_status: Option<String>,
    pub keyword: Option<String>,
    #[serde(default)]
    pub accounts: Vec<i64>,
    #[serde(default)]
    pub tags: Vec<i64>,
    pub calendar_type: Option<String>,
    pub date: Option<String>,
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ValidatedPostQuery {
    pub status: Option<String>,
    pub exclude_status: Option<String>,
    pub keyword: Option<String>,
    pub accounts: Vec<i64>,
    pub tags: Vec<i64>,
    pub calendar_type: Option<String>,
    pub date: Option<String>,
    pub limit: i64,
    pub page: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostForm {
    #[serde(default)]
    pub accounts: Vec<i64>,
    #[serde(default)]
    pub tags: Vec<i64>,
    pub scheduled_at: Option<String>,
    pub versions: Vec<PostVersionForm>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchedulePostForm {
    pub scheduled_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostVersionForm {
    pub account_id: i64,
    pub is_original: bool,
    pub content: Vec<PostContentBlock>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostContentBlock {
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub media: Vec<i64>,
    #[serde(default)]
    pub external_media: Vec<ExternalMediaItem>,
}

#[derive(Debug, Clone)]
pub struct ValidatedPost {
    pub accounts: Vec<i64>,
    pub tags: Vec<i64>,
    pub scheduled_at: Option<String>,
    pub versions: Vec<ValidatedPostVersion>,
}

#[derive(Debug, Clone)]
pub struct ValidatedPostVersion {
    pub account_id: i64,
    pub is_original: bool,
    pub content_json: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaSummary {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub mime_type: String,
    pub disk: String,
    pub path: String,
    pub size: i64,
    pub size_total: i64,
    pub conversion_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaLibraryItem {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub mime_type: String,
    pub media_type: String,
    pub url: Option<String>,
    pub thumb_url: Option<String>,
    pub is_video: bool,
    pub disk: String,
    pub path: String,
    pub size: i64,
    pub size_total: i64,
    pub conversion_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExternalMediaSearchRequest {
    pub source: String,
    pub keyword: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ValidatedExternalMediaSearch {
    pub source: String,
    pub keyword: Option<String>,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalMediaSearchResult {
    pub source: String,
    pub page: i64,
    pub next_page: i64,
    pub items: Vec<ExternalMediaItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExternalMediaItem {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub media_type: String,
    pub url: String,
    pub thumb_url: String,
    pub is_video: bool,
    pub credit_url: Option<String>,
    pub download_data: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MediaLibraryRequest {
    pub keyword: Option<String>,
    pub media_type: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ValidatedMediaLibraryQuery {
    pub keyword: Option<String>,
    pub media_type: Option<String>,
    pub limit: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MediaCleanupSummary {
    pub scanned: usize,
    pub retained: usize,
    pub deleted: usize,
    pub reclaimed_bytes: i64,
}

#[cfg(test)]
#[derive(Debug, Clone, Deserialize)]
pub struct MediaForm {
    pub name: String,
    pub mime_type: String,
    pub disk: String,
    pub path: String,
    pub size: i64,
    pub size_total: i64,
    pub data: Option<Value>,
    pub conversions: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MediaImportForm {
    pub source_path: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MediaDownloadForm {
    pub url: String,
    pub name: Option<String>,
    pub source: Option<String>,
    pub download_data: Option<Value>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct ValidatedMedia {
    pub name: String,
    pub mime_type: String,
    pub disk: String,
    pub path: String,
    pub size: i64,
    pub size_total: i64,
    pub data_json: Option<String>,
    pub conversions_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidatedMediaImport {
    pub source_path: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidatedMediaDownload {
    pub url: String,
    pub name: Option<String>,
    pub source: String,
    pub download_data: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagSummary {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub hex_color: String,
    pub post_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagForm {
    pub name: String,
    pub hex_color: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobSummary {
    pub id: i64,
    pub kind: String,
    pub status: String,
    pub attempts: i64,
    pub run_at: String,
    pub last_error: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct AccountImportQueueBatchSummary {
    pub requested_accounts: usize,
    pub eligible_accounts: usize,
    pub queued_jobs: usize,
    pub skipped_unsupported: usize,
    pub skipped_unauthorized: usize,
    pub jobs: Vec<JobSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkerRunSummary {
    pub now: String,
    pub limit: i64,
    pub reserved: usize,
    pub completed: usize,
    pub deferred: usize,
    pub failed: usize,
    pub outcomes: Vec<WorkerJobOutcome>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkerJobOutcome {
    pub job_id: i64,
    pub kind: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateLimitSummary {
    pub id: i64,
    pub scope: String,
    pub retry_after_at: String,
    pub payload: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(test)]
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitForm {
    pub scope: String,
    pub retry_after_at: String,
    pub payload: Option<Value>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct ValidatedRateLimit {
    pub scope: String,
    pub retry_after_at: String,
    pub payload_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReportRequest {
    pub account_id: i64,
    pub period: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricForm {
    pub account_id: i64,
    pub date: String,
    pub data: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AudienceForm {
    pub account_id: i64,
    pub date: String,
    pub total: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FacebookInsightForm {
    pub account_id: i64,
    pub insight_type: i64,
    pub date: String,
    pub value: i64,
}

#[derive(Debug, Clone)]
pub struct ValidatedMetric {
    pub account_id: i64,
    pub date: String,
    pub data_json: String,
}

#[derive(Debug, Clone)]
pub struct ValidatedAudience {
    pub account_id: i64,
    pub date: String,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct ValidatedFacebookInsight {
    pub account_id: i64,
    pub insight_type: i64,
    pub date: String,
    pub value: i64,
}

#[derive(Debug, Clone)]
pub struct ValidatedReportRequest {
    pub account_id: i64,
    pub period: String,
    pub days: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportSnapshot {
    pub account_id: i64,
    pub provider: String,
    pub period: String,
    pub tier: Option<String>,
    pub metrics: Vec<ReportMetric>,
    pub audience: AudienceReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportMetric {
    pub key: String,
    pub value: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudienceReport {
    pub labels: Vec<String>,
    pub values: Vec<Option<i64>>,
    pub points: Vec<AudiencePoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudiencePoint {
    pub date: String,
    pub label: String,
    pub value: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JobForm {
    pub kind: String,
    pub payload: Value,
    pub run_at: String,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidatedJob {
    pub kind: String,
    pub payload_json: String,
    pub run_at: String,
    pub idempotency_key: Option<String>,
}

impl TagForm {
    pub fn validated(&self) -> Result<ValidatedTag, String> {
        let name = self.name.trim();

        if name.is_empty() {
            return Err("name is required".to_string());
        }

        if name.chars().count() > 255 {
            return Err("name must be 255 characters or fewer".to_string());
        }

        Ok(ValidatedTag {
            name: name.to_string(),
            hex_color: normalize_hex_color(&self.hex_color)?,
        })
    }
}

impl AccountForm {
    pub fn validated(&self) -> Result<ValidatedAccount, String> {
        let name = self.name.trim();
        let provider = self.provider.trim();
        let provider_id = self.provider_id.trim();
        let access_token_secret_ref = self.access_token_secret_ref.trim();
        let data_json = self
            .data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| error.to_string())?;

        if name.is_empty() {
            return Err("name is required".to_string());
        }

        if provider.is_empty() {
            return Err("provider is required".to_string());
        }

        if provider_id.is_empty() {
            return Err("provider_id is required".to_string());
        }

        if access_token_secret_ref.is_empty() {
            return Err("access_token_secret_ref is required".to_string());
        }

        Ok(ValidatedAccount {
            name: name.to_string(),
            username: self
                .username
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            provider: provider.to_string(),
            provider_id: provider_id.to_string(),
            authorized: self.authorized,
            avatar_path: self
                .avatar_path
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            access_token_secret_ref: access_token_secret_ref.to_string(),
            data_json,
        })
    }
}

impl ServiceForm {
    pub fn validated(&self) -> Result<ValidatedService, String> {
        let name = self.name.trim();
        let configuration_secret_ref = self.configuration_secret_ref.trim();

        if name.is_empty() {
            return Err("name is required".to_string());
        }

        if name.chars().count() > 120 {
            return Err("name must be 120 characters or fewer".to_string());
        }

        if configuration_secret_ref.is_empty() {
            return Err("configuration_secret_ref is required".to_string());
        }

        validate_service_configuration(name, self.configuration.as_ref())?;

        Ok(ValidatedService {
            name: name.to_string(),
            configuration_secret_ref: configuration_secret_ref.to_string(),
            configuration_json: self
                .configuration
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| format!("configuration is invalid: {error}"))?,
            active: self.active,
        })
    }
}

fn validate_service_configuration(name: &str, configuration: Option<&Value>) -> Result<(), String> {
    let Some(configuration) = configuration else {
        return Ok(());
    };

    let Some(object) = configuration.as_object() else {
        return Err("configuration must be an object".to_string());
    };

    if name == "twitter" {
        if let Some(tier) = object.get("tier").and_then(Value::as_str) {
            if !matches!(tier, "legacy" | "free" | "basic" | "pay_as_you_go") {
                return Err("twitter tier is invalid".to_string());
            }
        }
    }

    if name == "facebook" {
        if let Some(api_version) = object.get("api_version").and_then(Value::as_str) {
            if !looks_like_facebook_api_version(api_version) {
                return Err("facebook api_version is invalid".to_string());
            }
        }
    }

    Ok(())
}

fn looks_like_facebook_api_version(value: &str) -> bool {
    value
        .strip_prefix('v')
        .and_then(|value| value.split_once('.'))
        .is_some_and(|(major, minor)| {
            !major.is_empty()
                && !minor.is_empty()
                && major.chars().all(|character| character.is_ascii_digit())
                && minor.chars().all(|character| character.is_ascii_digit())
        })
}

impl ServiceCredentialForm {
    pub fn validated(&self) -> Result<ValidatedServiceCredential, String> {
        let service = self.service.trim().to_lowercase();
        let field = self.field.trim().to_lowercase();
        let value = self.value.trim();

        if service.is_empty() {
            return Err("service is required".to_string());
        }

        if field.is_empty() {
            return Err("field is required".to_string());
        }

        if value.is_empty() {
            return Err("value is required".to_string());
        }

        Ok(ValidatedServiceCredential {
            service,
            field,
            value: value.to_string(),
        })
    }
}

impl LocalBackupRestoreForm {
    pub fn validated(&self) -> Result<ValidatedLocalBackupRestore, String> {
        let backup_path = self.backup_path.trim();

        if backup_path.is_empty() {
            return Err("backup_path is required".to_string());
        }

        Ok(ValidatedLocalBackupRestore {
            backup_path: backup_path.to_string(),
        })
    }
}

impl MastodonAppRegistrationForm {
    pub fn validated(&self) -> Result<ValidatedMastodonAppRegistration, String> {
        let server = normalize_mastodon_server(&self.server)?;
        let client_name = self
            .client_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("Dust Wave Social");
        let redirect_uri = self
            .redirect_uri
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("urn:ietf:wg:oauth:2.0:oob");
        let website = self
            .website
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        if client_name.chars().count() > 120 {
            return Err("client_name must be 120 characters or fewer".to_string());
        }

        Ok(ValidatedMastodonAppRegistration {
            server,
            client_name: client_name.to_string(),
            redirect_uri: redirect_uri.to_string(),
            website,
        })
    }
}

impl MastodonOAuthForm {
    pub fn validated(&self) -> Result<ValidatedMastodonOAuth, String> {
        let server = normalize_mastodon_server(&self.server)?;
        let code = self.code.trim();
        let redirect_uri = self
            .redirect_uri
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("urn:ietf:wg:oauth:2.0:oob");

        if code.is_empty() {
            return Err("code is required".to_string());
        }

        Ok(ValidatedMastodonOAuth {
            server,
            code: code.to_string(),
            redirect_uri: redirect_uri.to_string(),
        })
    }
}

#[cfg(test)]
impl MediaForm {
    pub fn validated(&self) -> Result<ValidatedMedia, String> {
        let name = self.name.trim();
        let mime_type = self.mime_type.trim();
        let disk = self.disk.trim();
        let path = self.path.trim();

        if name.is_empty() {
            return Err("name is required".to_string());
        }

        if mime_type.is_empty() {
            return Err("mime_type is required".to_string());
        }

        if disk.is_empty() {
            return Err("disk is required".to_string());
        }

        if path.is_empty() {
            return Err("path is required".to_string());
        }

        if self.size < 0 {
            return Err("size must be zero or greater".to_string());
        }

        if self.size_total < self.size {
            return Err("size_total must be greater than or equal to size".to_string());
        }

        Ok(ValidatedMedia {
            name: name.to_string(),
            mime_type: mime_type.to_string(),
            disk: disk.to_string(),
            path: path.to_string(),
            size: self.size,
            size_total: self.size_total,
            data_json: self
                .data
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| error.to_string())?,
            conversions_json: self
                .conversions
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| error.to_string())?,
        })
    }
}

impl MediaImportForm {
    pub fn validated(&self) -> Result<ValidatedMediaImport, String> {
        let source_path = self.source_path.trim();

        if source_path.is_empty() {
            return Err("source_path is required".to_string());
        }

        Ok(ValidatedMediaImport {
            source_path: source_path.to_string(),
            name: self
                .name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string),
        })
    }
}

impl MediaDownloadForm {
    pub fn validated(&self) -> Result<ValidatedMediaDownload, String> {
        let url = self.url.trim();

        if url.is_empty() {
            return Err("url is required".to_string());
        }

        if !url.starts_with("file://")
            && !url.starts_with("http://")
            && !url.starts_with("https://")
        {
            return Err("url must start with file://, http://, or https://".to_string());
        }

        Ok(ValidatedMediaDownload {
            url: url.to_string(),
            name: self
                .name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string),
            source: self
                .source
                .as_deref()
                .map(str::trim)
                .filter(|source| !source.is_empty())
                .unwrap_or("url")
                .to_string(),
            download_data: self.download_data.clone(),
        })
    }
}

impl ExternalMediaSearchRequest {
    pub fn validated(&self) -> Result<ValidatedExternalMediaSearch, String> {
        let source = self.source.trim().to_lowercase();

        if !matches!(source.as_str(), "stock" | "gifs") {
            return Err("source must be stock or gifs".to_string());
        }

        Ok(ValidatedExternalMediaSearch {
            source,
            keyword: self
                .keyword
                .as_deref()
                .map(str::trim)
                .filter(|keyword| !keyword.is_empty())
                .map(str::to_string),
            page: self.page.unwrap_or(1).max(1),
            limit: self.limit.unwrap_or(30).clamp(1, 30),
        })
    }
}

impl ExternalMediaItem {
    pub fn provider_key(&self) -> Option<&str> {
        self.download_data
            .as_ref()
            .and_then(|value| value.get("provider"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub fn is_klipy(&self) -> bool {
        self.provider_key()
            .is_some_and(|provider| provider.eq_ignore_ascii_case("klipy"))
    }

    pub fn validated_reference(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("external media id is required".to_string());
        }

        if !self.is_klipy() {
            return Err("only Klipy external media references are supported".to_string());
        }

        let url = self.url.trim();

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("external media url must start with http:// or https://".to_string());
        }

        let thumb_url = self.thumb_url.trim();

        if !thumb_url.is_empty()
            && !thumb_url.starts_with("http://")
            && !thumb_url.starts_with("https://")
        {
            return Err(
                "external media thumbnail url must start with http:// or https://".to_string(),
            );
        }

        if self.mime_type.trim() != "image/gif" || self.media_type.trim() != "gif" {
            return Err("Klipy external media references must be GIFs".to_string());
        }

        Ok(())
    }
}

impl MediaLibraryRequest {
    pub fn validated(&self) -> Result<ValidatedMediaLibraryQuery, String> {
        let keyword = self
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|keyword| !keyword.is_empty())
            .map(str::to_lowercase);
        let media_type = self
            .media_type
            .as_deref()
            .map(str::trim)
            .filter(|media_type| !media_type.is_empty())
            .map(str::to_lowercase);
        let limit = self.limit.unwrap_or(100).clamp(1, 200);

        if let Some(media_type) = media_type.as_deref() {
            if !matches!(media_type, "image" | "gif" | "video" | "file") {
                return Err("media_type must be image, gif, video, or file".to_string());
            }
        }

        Ok(ValidatedMediaLibraryQuery {
            keyword,
            media_type,
            limit,
        })
    }
}

impl ReportRequest {
    pub fn validated(&self) -> Result<ValidatedReportRequest, String> {
        if self.account_id <= 0 {
            return Err("account_id must be positive".to_string());
        }

        let days = match self.period.as_str() {
            "7_days" => 7,
            "30_days" => 30,
            "90_days" => 90,
            _ => return Err("period must be one of 7_days, 30_days, or 90_days".to_string()),
        };

        Ok(ValidatedReportRequest {
            account_id: self.account_id,
            period: self.period.clone(),
            days,
        })
    }
}

impl MetricForm {
    pub fn validated(&self) -> Result<ValidatedMetric, String> {
        validate_account_and_date(self.account_id, &self.date).map(|date| ValidatedMetric {
            account_id: self.account_id,
            date,
            data_json: serde_json::to_string(&self.data).expect("serde_json::Value serializes"),
        })
    }
}

impl AudienceForm {
    pub fn validated(&self) -> Result<ValidatedAudience, String> {
        if self.total < 0 {
            return Err("total must be zero or greater".to_string());
        }

        validate_account_and_date(self.account_id, &self.date).map(|date| ValidatedAudience {
            account_id: self.account_id,
            date,
            total: self.total,
        })
    }
}

impl FacebookInsightForm {
    pub fn validated(&self) -> Result<ValidatedFacebookInsight, String> {
        if !matches!(self.insight_type, 1 | 2 | 3) {
            return Err("insight_type must be 1, 2, or 3".to_string());
        }

        if self.value < 0 {
            return Err("value must be zero or greater".to_string());
        }

        validate_account_and_date(self.account_id, &self.date).map(|date| {
            ValidatedFacebookInsight {
                account_id: self.account_id,
                insight_type: self.insight_type,
                date,
                value: self.value,
            }
        })
    }
}

impl JobForm {
    pub fn validated(&self) -> Result<ValidatedJob, String> {
        let kind = self.kind.trim();
        let run_at = self.run_at.trim();

        if kind.is_empty() {
            return Err("kind is required".to_string());
        }

        if kind.chars().count() > 120 {
            return Err("kind must be 120 characters or fewer".to_string());
        }

        if run_at.is_empty() {
            return Err("run_at is required".to_string());
        }

        let idempotency_key = self
            .idempotency_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        if idempotency_key
            .as_deref()
            .is_some_and(|value| value.chars().count() > 180)
        {
            return Err("idempotency key must be 180 characters or fewer".to_string());
        }

        Ok(ValidatedJob {
            kind: kind.to_string(),
            payload_json: serde_json::to_string(&self.payload)
                .map_err(|error| error.to_string())?,
            run_at: run_at.to_string(),
            idempotency_key,
        })
    }
}

#[cfg(test)]
impl RateLimitForm {
    pub fn validated(&self) -> Result<ValidatedRateLimit, String> {
        let scope = self.scope.trim();
        let retry_after_at = self.retry_after_at.trim();

        if scope.is_empty() {
            return Err("scope is required".to_string());
        }

        if scope.chars().count() > 160 {
            return Err("scope must be 160 characters or fewer".to_string());
        }

        if retry_after_at.is_empty() {
            return Err("retry_after_at is required".to_string());
        }

        Ok(ValidatedRateLimit {
            scope: scope.to_string(),
            retry_after_at: retry_after_at.to_string(),
            payload_json: self
                .payload
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| error.to_string())?,
        })
    }
}

impl PostQueryRequest {
    pub fn validated(&self) -> Result<ValidatedPostQuery, String> {
        let status = self
            .status
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .map(validate_post_status_filter)
            .transpose()?;
        let exclude_status = self
            .exclude_status
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .map(validate_post_status_filter)
            .transpose()?;
        let keyword = self
            .keyword
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);

        if keyword
            .as_ref()
            .is_some_and(|value| value.chars().count() > 255)
        {
            return Err("keyword must be 255 characters or fewer".to_string());
        }

        if self.accounts.iter().any(|id| *id <= 0) {
            return Err("accounts must contain positive ids".to_string());
        }

        if self.tags.iter().any(|id| *id <= 0) {
            return Err("tags must contain positive ids".to_string());
        }

        let calendar_type = self
            .calendar_type
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .map(validate_calendar_type)
            .transpose()?;
        let date = self
            .date
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);

        if calendar_type.is_some() && date.is_none() {
            return Err("date is required when calendar_type is provided".to_string());
        }

        if date.as_ref().is_some_and(|value| !looks_like_date(value)) {
            return Err("date must use YYYY-MM-DD format".to_string());
        }

        Ok(ValidatedPostQuery {
            status,
            exclude_status,
            keyword,
            accounts: self.accounts.clone(),
            tags: self.tags.clone(),
            calendar_type,
            date,
            limit: self.limit.unwrap_or(50).clamp(1, 200),
            page: self.page.unwrap_or(1).max(1),
        })
    }
}

impl PostForm {
    pub fn validated(&self) -> Result<ValidatedPost, String> {
        if self.versions.is_empty() {
            return Err("versions must contain at least one item".to_string());
        }

        if self.accounts.iter().any(|id| *id <= 0) {
            return Err("accounts must contain positive ids".to_string());
        }

        if self.tags.iter().any(|id| *id <= 0) {
            return Err("tags must contain positive ids".to_string());
        }

        let scheduled_at = self
            .scheduled_at
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);

        let mut versions = Vec::with_capacity(self.versions.len());

        for version in &self.versions {
            if version.account_id < 0 {
                return Err("version account_id must be zero or a positive account id".to_string());
            }

            if version.content.is_empty() {
                return Err("version content must contain at least one item".to_string());
            }

            for item in &version.content {
                if item.body.chars().count() > 5000 {
                    return Err("version content body must be 5000 characters or fewer".to_string());
                }

                if item.media.iter().any(|id| *id <= 0) {
                    return Err("version media must contain positive ids".to_string());
                }

                for external_media in &item.external_media {
                    external_media.validated_reference()?;
                }
            }

            versions.push(ValidatedPostVersion {
                account_id: version.account_id,
                is_original: version.is_original,
                content_json: serde_json::to_string(&version.content)
                    .map_err(|error| error.to_string())?,
            });
        }

        Ok(ValidatedPost {
            accounts: self.accounts.clone(),
            tags: self.tags.clone(),
            scheduled_at,
            versions,
        })
    }
}

impl SchedulePostForm {
    pub fn validated(&self) -> Result<String, String> {
        let scheduled_at = self.scheduled_at.trim();

        if scheduled_at.is_empty() {
            return Err("scheduled_at is required".to_string());
        }

        Ok(scheduled_at.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedTag {
    pub name: String,
    pub hex_color: String,
}

pub fn post_status_label(status: i64, schedule_status: i64) -> &'static str {
    if schedule_status == 1 {
        return "publishing";
    }

    match status {
        0 => "draft",
        1 => "scheduled",
        2 => "published",
        3 => "failed",
        _ => "unknown",
    }
}

pub fn post_schedule_status_label(schedule_status: i64) -> &'static str {
    match schedule_status {
        0 => "pending",
        1 => "processing",
        2 => "processed",
        _ => "unknown",
    }
}

pub fn post_preview(content_json: Option<String>) -> String {
    let Some(content_json) = content_json else {
        return String::new();
    };

    let Ok(value) = serde_json::from_str::<Value>(&content_json) else {
        return String::new();
    };

    let body = value
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("body"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    truncate(&strip_html(body), 150)
}

pub fn media_conversion_count(conversions_json: Option<String>) -> usize {
    let Some(conversions_json) = conversions_json else {
        return 0;
    };

    let Ok(value) = serde_json::from_str::<Value>(&conversions_json) else {
        return 0;
    };

    match value {
        Value::Array(items) => items.len(),
        Value::Object(items) => items.len(),
        _ => 0,
    }
}

fn normalize_hex_color(value: &str) -> Result<String, String> {
    let color = value.trim().trim_start_matches('#');

    if !matches!(color.len(), 3 | 6)
        || !color.chars().all(|character| character.is_ascii_hexdigit())
    {
        return Err("hex_color must be a 3 or 6 character hex color".to_string());
    }

    if color.len() == 3 {
        let expanded = color
            .chars()
            .flat_map(|character| [character, character])
            .collect::<String>();

        return Ok(expanded.to_ascii_lowercase());
    }

    Ok(color.to_ascii_lowercase())
}

fn validate_account_and_date(account_id: i64, date: &str) -> Result<String, String> {
    if account_id <= 0 {
        return Err("account_id must be positive".to_string());
    }

    let date = date.trim();

    if date.is_empty() {
        return Err("date is required".to_string());
    }

    Ok(date.to_string())
}

fn validate_post_status_filter(value: String) -> Result<String, String> {
    if matches!(
        value.as_str(),
        "draft" | "scheduled" | "published" | "failed"
    ) {
        return Ok(value);
    }

    Err("status must be draft, scheduled, published, or failed".to_string())
}

fn validate_calendar_type(value: String) -> Result<String, String> {
    if matches!(value.as_str(), "month" | "week" | "day") {
        return Ok(value);
    }

    Err("calendar_type must be month, week, or day".to_string())
}

fn looks_like_date(value: &str) -> bool {
    let bytes = value.as_bytes();

    value.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn normalize_mastodon_server(value: &str) -> Result<String, String> {
    let mut server = value.trim();

    if let Some(stripped) = server.strip_prefix("https://") {
        server = stripped;
    } else if let Some(stripped) = server.strip_prefix("http://") {
        server = stripped;
    }

    server = server.trim_matches('/');

    if server.is_empty() {
        return Err("server is required".to_string());
    }

    if server.chars().count() > 255 {
        return Err("server must be 255 characters or fewer".to_string());
    }

    if format!("mastodon.{server}").chars().count() > 120 {
        return Err("server is too long for a local Mastodon service name".to_string());
    }

    if server.contains('/') || server.chars().any(char::is_whitespace) {
        return Err("server must be a Mastodon host name without a path".to_string());
    }

    Ok(server.to_lowercase())
}

fn strip_html(value: &str) -> String {
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

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut truncated = value.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");

    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_preview_from_mixpost_content_json() {
        let preview = post_preview(Some(
            r#"[{"body":"<div>First line</div><div>Second line</div>","media":[1]}]"#.to_string(),
        ));

        assert_eq!(preview, "First line Second line");
    }

    #[test]
    fn counts_array_and_object_conversions() {
        assert_eq!(
            media_conversion_count(Some(r#"["thumb","small"]"#.to_string())),
            2
        );
        assert_eq!(
            media_conversion_count(Some(r#"{"thumb":{},"small":{}}"#.to_string())),
            2
        );
        assert_eq!(media_conversion_count(None), 0);
    }

    #[test]
    fn validates_and_normalizes_tag_forms() {
        let tag = TagForm {
            name: " Launch ".to_string(),
            hex_color: "#ABC".to_string(),
        }
        .validated()
        .expect("tag should validate");

        assert_eq!(tag.name, "Launch");
        assert_eq!(tag.hex_color, "aabbcc");
    }

    #[test]
    fn validates_account_forms() {
        let account = AccountForm {
            name: " Dust Wave ".to_string(),
            username: Some(" dustwave ".to_string()),
            provider: " mastodon ".to_string(),
            provider_id: " dw-1 ".to_string(),
            authorized: true,
            avatar_path: None,
            access_token_secret_ref: " secret://accounts/1 ".to_string(),
            data: Some(serde_json::json!({ "server": "mastodon.social" })),
        }
        .validated()
        .expect("account should validate");

        assert_eq!(account.name, "Dust Wave");
        assert_eq!(account.username.as_deref(), Some("dustwave"));
        assert_eq!(account.provider, "mastodon");
        assert_eq!(account.provider_id, "dw-1");
        assert_eq!(account.access_token_secret_ref, "secret://accounts/1");
        assert_eq!(
            account.data_json.as_deref(),
            Some(r#"{"server":"mastodon.social"}"#)
        );
    }

    #[test]
    fn validates_service_forms() {
        let service = ServiceForm {
            name: " mastodon ".to_string(),
            configuration_secret_ref: " secret://services/mastodon ".to_string(),
            configuration: Some(serde_json::json!({ "tier": "free" })),
            active: true,
        }
        .validated()
        .expect("service should validate");

        assert_eq!(service.name, "mastodon");
        assert_eq!(
            service.configuration_secret_ref,
            "secret://services/mastodon"
        );
        assert_eq!(
            service.configuration_json.as_deref(),
            Some(r#"{"tier":"free"}"#)
        );
        assert!(service.active);

        let error = ServiceForm {
            name: "twitter".to_string(),
            configuration_secret_ref: "secret://services/twitter".to_string(),
            configuration: Some(serde_json::json!({ "tier": "enterprise" })),
            active: true,
        }
        .validated()
        .expect_err("unsupported twitter tier should fail");
        assert_eq!(error, "twitter tier is invalid");

        let error = ServiceForm {
            name: "facebook".to_string(),
            configuration_secret_ref: "secret://services/facebook".to_string(),
            configuration: Some(serde_json::json!({ "api_version": "25" })),
            active: true,
        }
        .validated()
        .expect_err("bad facebook version should fail");
        assert_eq!(error, "facebook api_version is invalid");
    }

    #[test]
    fn validates_service_credential_forms() {
        let credential = ServiceCredentialForm {
            service: " Unsplash ".to_string(),
            field: " Client_ID ".to_string(),
            value: " secret-value ".to_string(),
        }
        .validated()
        .expect("credential should validate");

        assert_eq!(credential.service, "unsplash");
        assert_eq!(credential.field, "client_id");
        assert_eq!(credential.value, "secret-value");

        let error = ServiceCredentialForm {
            service: "unsplash".to_string(),
            field: "client_id".to_string(),
            value: " ".to_string(),
        }
        .validated()
        .expect_err("empty credential should fail");

        assert_eq!(error, "value is required");
    }

    #[test]
    fn validates_mastodon_app_registration_forms() {
        let request = MastodonAppRegistrationForm {
            server: " https://Mastodon.Social/ ".to_string(),
            client_name: None,
            redirect_uri: None,
            website: Some(" https://dustwave.example ".to_string()),
        }
        .validated()
        .expect("mastodon app request should validate");

        assert_eq!(request.server, "mastodon.social");
        assert_eq!(request.client_name, "Dust Wave Social");
        assert_eq!(request.redirect_uri, "urn:ietf:wg:oauth:2.0:oob");
        assert_eq!(request.website.as_deref(), Some("https://dustwave.example"));

        let error = MastodonAppRegistrationForm {
            server: "mastodon.social/path".to_string(),
            client_name: None,
            redirect_uri: None,
            website: None,
        }
        .validated()
        .expect_err("path should fail");

        assert_eq!(error, "server must be a Mastodon host name without a path");
    }

    #[test]
    fn validates_mastodon_oauth_forms() {
        let request = MastodonOAuthForm {
            server: " https://Mastodon.Social/ ".to_string(),
            code: " auth-code ".to_string(),
            redirect_uri: None,
        }
        .validated()
        .expect("mastodon oauth request should validate");

        assert_eq!(request.server, "mastodon.social");
        assert_eq!(request.code, "auth-code");
        assert_eq!(request.redirect_uri, "urn:ietf:wg:oauth:2.0:oob");

        let error = MastodonOAuthForm {
            server: "mastodon.social".to_string(),
            code: " ".to_string(),
            redirect_uri: None,
        }
        .validated()
        .expect_err("empty oauth code should fail");

        assert_eq!(error, "code is required");
    }

    #[test]
    fn validates_media_forms() {
        let media = MediaForm {
            name: " launch.png ".to_string(),
            mime_type: " image/png ".to_string(),
            disk: " local ".to_string(),
            path: " media/launch.png ".to_string(),
            size: 100,
            size_total: 150,
            data: Some(serde_json::json!({ "source": "upload" })),
            conversions: Some(serde_json::json!({ "thumb": {} })),
        }
        .validated()
        .expect("media should validate");

        assert_eq!(media.name, "launch.png");
        assert_eq!(media.mime_type, "image/png");
        assert_eq!(media.data_json.as_deref(), Some(r#"{"source":"upload"}"#));
        assert_eq!(media.conversions_json.as_deref(), Some(r#"{"thumb":{}}"#));
    }

    #[test]
    fn validates_media_import_forms() {
        let media = MediaImportForm {
            source_path: " /tmp/launch.png ".to_string(),
            name: Some(" Launch Asset ".to_string()),
        }
        .validated()
        .expect("media import should validate");

        assert_eq!(media.source_path, "/tmp/launch.png");
        assert_eq!(media.name.as_deref(), Some("Launch Asset"));

        let media = MediaImportForm {
            source_path: "/tmp/launch.png".to_string(),
            name: Some(" ".to_string()),
        }
        .validated()
        .expect("empty name should be optional");

        assert_eq!(media.name, None);
    }

    #[test]
    fn validates_media_download_forms() {
        let media = MediaDownloadForm {
            url: " file:///tmp/launch.gif ".to_string(),
            name: Some(" Launch GIF ".to_string()),
            source: Some(" gifs ".to_string()),
            download_data: Some(serde_json::json!({ "provider": "tenor" })),
        }
        .validated()
        .expect("media download should validate");

        assert_eq!(media.url, "file:///tmp/launch.gif");
        assert_eq!(media.name.as_deref(), Some("Launch GIF"));
        assert_eq!(media.source, "gifs");
        assert_eq!(
            media.download_data.as_ref(),
            Some(&serde_json::json!({ "provider": "tenor" }))
        );

        let error = MediaDownloadForm {
            url: "/tmp/launch.gif".to_string(),
            name: None,
            source: None,
            download_data: None,
        }
        .validated()
        .expect_err("non-url download should fail");

        assert_eq!(error, "url must start with file://, http://, or https://");
    }

    #[test]
    fn validates_klipy_external_media_references_in_post_content() {
        let post = PostForm {
            accounts: vec![1],
            tags: vec![],
            scheduled_at: None,
            versions: vec![PostVersionForm {
                account_id: 0,
                is_original: true,
                content: vec![PostContentBlock {
                    body: "".to_string(),
                    media: vec![],
                    external_media: vec![ExternalMediaItem {
                        id: "wave".to_string(),
                        name: "Wave".to_string(),
                        mime_type: "image/gif".to_string(),
                        media_type: "gif".to_string(),
                        url: "https://cdn.klipy.com/wave.gif".to_string(),
                        thumb_url: "https://cdn.klipy.com/wave-thumb.gif".to_string(),
                        is_video: false,
                        credit_url: None,
                        download_data: Some(serde_json::json!({ "provider": "klipy" })),
                    }],
                }],
            }],
        }
        .validated()
        .expect("klipy external media reference should validate");

        assert!(post.versions[0].content_json.contains("external_media"));

        let error = PostForm {
            accounts: vec![1],
            tags: vec![],
            scheduled_at: None,
            versions: vec![PostVersionForm {
                account_id: 0,
                is_original: true,
                content: vec![PostContentBlock {
                    body: "".to_string(),
                    media: vec![],
                    external_media: vec![ExternalMediaItem {
                        id: "wave".to_string(),
                        name: "Wave".to_string(),
                        mime_type: "image/gif".to_string(),
                        media_type: "gif".to_string(),
                        url: "https://cdn.example.com/wave.gif".to_string(),
                        thumb_url: "https://cdn.example.com/wave-thumb.gif".to_string(),
                        is_video: false,
                        credit_url: None,
                        download_data: Some(serde_json::json!({ "provider": "other" })),
                    }],
                }],
            }],
        }
        .validated()
        .expect_err("non-klipy external media should fail");

        assert_eq!(error, "only Klipy external media references are supported");
    }

    #[test]
    fn validates_external_media_search_requests() {
        let request = ExternalMediaSearchRequest {
            source: " Stock ".to_string(),
            keyword: Some(" Wave ".to_string()),
            page: Some(0),
            limit: Some(100),
        }
        .validated()
        .expect("external media search should validate");

        assert_eq!(request.source, "stock");
        assert_eq!(request.keyword.as_deref(), Some("Wave"));
        assert_eq!(request.page, 1);
        assert_eq!(request.limit, 30);

        let request = ExternalMediaSearchRequest {
            source: "gifs".to_string(),
            keyword: Some(" ".to_string()),
            page: None,
            limit: None,
        }
        .validated()
        .expect("gif search should validate");

        assert_eq!(request.keyword, None);
        assert_eq!(request.page, 1);
        assert_eq!(request.limit, 30);

        let error = ExternalMediaSearchRequest {
            source: "videos".to_string(),
            keyword: None,
            page: None,
            limit: None,
        }
        .validated()
        .expect_err("unsupported source should fail");

        assert_eq!(error, "source must be stock or gifs");
    }

    #[test]
    fn validates_media_library_requests() {
        let request = MediaLibraryRequest {
            keyword: Some(" Launch ".to_string()),
            media_type: Some(" GIF ".to_string()),
            limit: Some(500),
        }
        .validated()
        .expect("media library request should validate");

        assert_eq!(request.keyword.as_deref(), Some("launch"));
        assert_eq!(request.media_type.as_deref(), Some("gif"));
        assert_eq!(request.limit, 200);

        let request = MediaLibraryRequest {
            keyword: Some(" ".to_string()),
            media_type: None,
            limit: Some(0),
        }
        .validated()
        .expect("empty keyword should be ignored");

        assert_eq!(request.keyword, None);
        assert_eq!(request.limit, 1);

        let error = MediaLibraryRequest {
            keyword: None,
            media_type: Some("audio".to_string()),
            limit: None,
        }
        .validated()
        .expect_err("unsupported media type should fail");

        assert_eq!(error, "media_type must be image, gif, video, or file");
    }

    #[test]
    fn validates_report_requests() {
        let request = ReportRequest {
            account_id: 1,
            period: "30_days".to_string(),
        }
        .validated()
        .expect("report request should validate");

        assert_eq!(request.account_id, 1);
        assert_eq!(request.days, 30);
    }

    #[test]
    fn validates_metric_ingest_forms() {
        let metric = MetricForm {
            account_id: 1,
            date: " 2026-06-22 ".to_string(),
            data: serde_json::json!({ "likes": 4 }),
        }
        .validated()
        .expect("metric should validate");

        assert_eq!(metric.date, "2026-06-22");
        assert_eq!(metric.data_json, r#"{"likes":4}"#);
    }

    #[test]
    fn validates_audience_ingest_forms() {
        let audience = AudienceForm {
            account_id: 1,
            date: " 2026-06-22 ".to_string(),
            total: 1200,
        }
        .validated()
        .expect("audience should validate");

        assert_eq!(audience.date, "2026-06-22");
        assert_eq!(audience.total, 1200);
    }

    #[test]
    fn validates_facebook_insight_ingest_forms() {
        let insight = FacebookInsightForm {
            account_id: 1,
            insight_type: 2,
            date: " 2026-06-22 ".to_string(),
            value: 20,
        }
        .validated()
        .expect("insight should validate");

        assert_eq!(insight.date, "2026-06-22");
        assert_eq!(insight.insight_type, 2);
    }

    #[test]
    fn validates_job_forms() {
        let job = JobForm {
            kind: " publish_post ".to_string(),
            payload: serde_json::json!({ "post_id": 1 }),
            run_at: "2026-06-24T15:00:00Z".to_string(),
            idempotency_key: Some(" publish-post:1 ".to_string()),
        }
        .validated()
        .expect("job should validate");

        assert_eq!(job.kind, "publish_post");
        assert_eq!(job.payload_json, r#"{"post_id":1}"#);
        assert_eq!(job.idempotency_key.as_deref(), Some("publish-post:1"));
    }

    #[test]
    fn validates_rate_limit_forms() {
        let rate_limit = RateLimitForm {
            scope: " mixpost-mastodon-api-limit ".to_string(),
            retry_after_at: " 2026-06-24T16:00:00Z ".to_string(),
            payload: Some(serde_json::json!({ "reason": "provider throttle" })),
        }
        .validated()
        .expect("rate limit should validate");

        assert_eq!(rate_limit.scope, "mixpost-mastodon-api-limit");
        assert_eq!(rate_limit.retry_after_at, "2026-06-24T16:00:00Z");
        assert_eq!(
            rate_limit.payload_json.as_deref(),
            Some(r#"{"reason":"provider throttle"}"#)
        );
    }

    #[test]
    fn validates_post_query_requests() {
        let query = PostQueryRequest {
            status: Some(" Scheduled ".to_string()),
            exclude_status: None,
            keyword: Some(" launch ".to_string()),
            accounts: vec![1],
            tags: vec![2],
            calendar_type: Some(" Month ".to_string()),
            date: Some("2026-06-24".to_string()),
            limit: Some(500),
            page: Some(2),
        }
        .validated()
        .expect("post query should validate");

        assert_eq!(query.status.as_deref(), Some("scheduled"));
        assert_eq!(query.keyword.as_deref(), Some("launch"));
        assert_eq!(query.calendar_type.as_deref(), Some("month"));
        assert_eq!(query.limit, 200);
        assert_eq!(query.page, 2);

        let error = PostQueryRequest {
            status: Some("queued".to_string()),
            exclude_status: None,
            keyword: None,
            accounts: vec![],
            tags: vec![],
            calendar_type: None,
            date: None,
            limit: None,
            page: None,
        }
        .validated()
        .expect_err("invalid status should fail");

        assert_eq!(
            error,
            "status must be draft, scheduled, published, or failed"
        );
    }

    #[test]
    fn validates_post_forms() {
        let post = PostForm {
            accounts: vec![],
            tags: vec![1],
            scheduled_at: Some(" 2026-06-24T15:00:00Z ".to_string()),
            versions: vec![PostVersionForm {
                account_id: 0,
                is_original: true,
                content: vec![PostContentBlock {
                    body: "Draft body".to_string(),
                    media: vec![],
                    external_media: vec![],
                }],
            }],
        }
        .validated()
        .expect("post should validate");

        assert_eq!(post.tags, vec![1]);
        assert_eq!(post.scheduled_at.as_deref(), Some("2026-06-24T15:00:00Z"));
        assert_eq!(
            post.versions[0].content_json,
            r#"[{"body":"Draft body","media":[],"external_media":[]}]"#
        );
    }

    #[test]
    fn validates_schedule_post_forms() {
        let scheduled_at = SchedulePostForm {
            scheduled_at: " 2026-06-24T15:00:00Z ".to_string(),
        }
        .validated()
        .expect("schedule form should validate");

        assert_eq!(scheduled_at, "2026-06-24T15:00:00Z");
    }
}
