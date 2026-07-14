use crate::db::Database;
use crate::domain::{
    AccountImportQueueBatchSummary, AccountSummary, AppSettings, AudienceForm, BulkDeletePostsForm,
    BulkDeletePostsSummary, DashboardSummary, DesktopMaintenanceSummary,
    ExternalMediaSearchRequest, ExternalMediaSearchResult, FacebookImportSummary,
    FacebookInsightForm, FacebookOAuthExchangeForm, FacebookOAuthStartForm,
    FacebookOAuthStartSummary, FacebookPageConnectForm, FacebookPageConnectionSummary,
    FacebookUserConnectionSummary, JobSummary, LocalBackupExportSummary, LocalBackupRestoreForm,
    LocalBackupRestoreSummary, LocalDataSnapshot, MastodonAccountConnection,
    MastodonAppRegistrationForm, MastodonAppRegistrationSummary, MastodonImportSummary,
    MastodonOAuthForm, MediaCleanupSummary, MediaDownloadForm, MediaImportForm, MediaLibraryItem,
    MediaLibraryRequest, MediaSummary, MetricForm, PostDetail, PostForm, PostQueryRequest,
    PostQueryResult, PostSummary, PostValidationReport, ReportRequest, ReportSnapshot,
    SchedulePostForm, ServiceCredentialForm, ServiceCredentialStatus, ServiceForm, ServiceSummary,
    StaleJobRecoverySummary, SystemHealthCounts, SystemHealthIssue, SystemHealthSummary,
    SystemLogClearSummary, SystemLogExportSummary, SystemLogFile, SystemMaintenanceSummary,
    SystemMediaToolStatus, SystemMediaToolSummary, TagForm, TagSummary, TwitterAccountConnection,
    TwitterImportSummary, TwitterOAuthExchangeForm, TwitterOAuthStartForm,
    TwitterOAuthStartSummary, WorkerRunSummary,
};
use crate::external_media::search_external_media as search_external_media_provider;
use crate::facebook::{
    connect_facebook_pages as connect_facebook_pages_provider,
    exchange_facebook_oauth as exchange_facebook_oauth_provider,
    start_facebook_oauth as start_facebook_oauth_provider,
};
use crate::mastodon::{
    connect_mastodon_account as connect_mastodon_account_provider,
    register_mastodon_app as register_mastodon_app_provider,
};
use crate::media_tools::{self, MediaToolSource};
use crate::secrets::{
    save_service_credential as save_service_credential_secret,
    service_credential_statuses as build_service_credential_statuses,
};
use crate::twitter::{
    connect_twitter_account as connect_twitter_account_provider,
    start_twitter_oauth as start_twitter_oauth_provider,
};
use chrono::Utc;
use tauri::{AppHandle, Manager, State};

fn build_system_health_summary(
    counts: SystemHealthCounts,
    credential_statuses: &[ServiceCredentialStatus],
    generated_at: String,
) -> SystemHealthSummary {
    let mut issues = Vec::new();
    let missing_credentials = missing_active_credentials(credential_statuses);

    if counts.unauthorized_accounts > 0 {
        issues.push(SystemHealthIssue {
            severity: "error".to_string(),
            title: "Unauthorized accounts".to_string(),
            detail: format!(
                "{} account(s) need reconnection before publishing or importing.",
                counts.unauthorized_accounts
            ),
        });
    }

    if counts.failed_posts > 0 {
        issues.push(SystemHealthIssue {
            severity: "error".to_string(),
            title: "Failed posts".to_string(),
            detail: format!(
                "{} post(s) failed and should be reviewed or retried.",
                counts.failed_posts
            ),
        });
    }

    if counts.failed_jobs > 0 {
        issues.push(SystemHealthIssue {
            severity: "error".to_string(),
            title: "Failed background work".to_string(),
            detail: format!(
                "{} background item(s) failed. Review the system log before clearing resolved state.",
                counts.failed_jobs
            ),
        });
    }

    if counts.processing_jobs > 0 {
        issues.push(SystemHealthIssue {
            severity: "warning".to_string(),
            title: "Processing background work".to_string(),
            detail: format!(
                "{} item(s) are marked processing. Recover stale items if this count does not clear.",
                counts.processing_jobs
            ),
        });
    }

    if !missing_credentials.is_empty() {
        issues.push(SystemHealthIssue {
            severity: "error".to_string(),
            title: "Missing active service credentials".to_string(),
            detail: missing_credentials.join(", "),
        });
    }

    if counts.rate_limits > 0 {
        issues.push(SystemHealthIssue {
            severity: "warning".to_string(),
            title: "Active rate limits".to_string(),
            detail: format!(
                "{} provider limit(s) are delaying queued publishing or imports.",
                counts.rate_limits
            ),
        });
    }

    let status = if issues.iter().any(|issue| issue.severity == "error") {
        "needs_attention"
    } else if issues.is_empty() {
        "ok"
    } else {
        "warning"
    };

    SystemHealthSummary {
        generated_at,
        status: status.to_string(),
        counts,
        issues,
        media_tools: system_media_tool_summary(),
    }
}

fn system_media_tool_summary() -> SystemMediaToolSummary {
    SystemMediaToolSummary {
        ffmpeg: media_tool_status("FFmpeg", "FFMPEG_PATH", "ffmpeg"),
        ffprobe: media_tool_status("FFprobe", "FFPROBE_PATH", "ffprobe"),
    }
}

fn media_tool_status(name: &str, env_var: &str, fallback_command: &str) -> SystemMediaToolStatus {
    let resolution = media_tools::resolve_media_tool(env_var, fallback_command);
    let detail = match (resolution.available, resolution.source) {
        (true, MediaToolSource::ConfiguredEnv) => {
            format!("{name} is available through {env_var}.")
        }
        (true, MediaToolSource::Bundled) => {
            format!("{name} is available from the app bundle.")
        }
        (true, MediaToolSource::SystemPath) => {
            format!("{name} is available at {}.", resolution.command)
        }
        (true, MediaToolSource::Path) => {
            format!("{name} is available on PATH.")
        }
        (false, MediaToolSource::ConfiguredEnv) => {
            format!("{name} was configured through {env_var}, but the command could not run.")
        }
        (false, MediaToolSource::Bundled) => {
            format!("{name} was found in the app bundle, but the command could not run.")
        }
        (false, _) => {
            format!(
                "{name} was not found. Video uploads still work, but thumbnails may not generate."
            )
        }
    };

    SystemMediaToolStatus {
        name: name.to_string(),
        command: resolution.command,
        available: resolution.available,
        detail,
    }
}

fn missing_active_credentials(statuses: &[ServiceCredentialStatus]) -> Vec<String> {
    statuses
        .iter()
        .filter(|status| status.active && !status.configured)
        .flat_map(|status| {
            status
                .fields
                .iter()
                .filter(|field| !field.configured)
                .map(|field| format!("{} {}", status.label, field.label))
        })
        .collect()
}

fn service_configuration_value(
    database: &Database,
    service: &str,
    key: &str,
) -> Result<Option<String>, String> {
    database
        .service_configuration_value(service, key)
        .map_err(|error| error.to_string())
}

fn facebook_api_version(database: &Database) -> Result<Option<String>, String> {
    service_configuration_value(database, "facebook", "api_version")
}

#[tauri::command]
pub fn system_health(database: State<'_, Database>) -> Result<SystemHealthSummary, String> {
    let counts = database
        .system_health_counts()
        .map_err(|error| error.to_string())?;
    let services = database.services().map_err(|error| error.to_string())?;
    let credential_statuses = build_service_credential_statuses(&services);

    Ok(build_system_health_summary(
        counts,
        &credential_statuses,
        Utc::now().to_rfc3339(),
    ))
}

#[tauri::command]
pub fn dashboard_summary(database: State<'_, Database>) -> Result<DashboardSummary, String> {
    database
        .dashboard_summary(&Utc::now().to_rfc3339())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn app_data_directory(app: AppHandle) -> Result<String, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.display().to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_resolved_system_state(
    database: State<'_, Database>,
    now: String,
) -> Result<SystemMaintenanceSummary, String> {
    let summary = database
        .clear_resolved_system_state(&now)
        .map_err(|error| error.to_string())?;

    let _ = database.record_system_log(
        "info",
        "Cleared resolved system state",
        Some(serde_json::json!({
            "completed_jobs_deleted": summary.completed_jobs_deleted,
            "cancelled_jobs_deleted": summary.cancelled_jobs_deleted,
            "expired_rate_limits_cleared": summary.expired_rate_limits_cleared
        })),
    );

    Ok(summary)
}

#[tauri::command]
pub fn run_desktop_maintenance(
    database: State<'_, Database>,
    now: String,
) -> Result<DesktopMaintenanceSummary, String> {
    let resolved_state = database
        .clear_resolved_system_state(&now)
        .map_err(|error| error.to_string())?;
    let media = database
        .cleanup_orphaned_media_files()
        .map_err(|error| error.to_string())?;

    let summary = DesktopMaintenanceSummary {
        now: now.clone(),
        resolved_state,
        media,
    };

    let _ = database.record_system_log(
        "info",
        "Ran desktop maintenance",
        Some(serde_json::json!({
            "completed_jobs_deleted": summary.resolved_state.completed_jobs_deleted,
            "cancelled_jobs_deleted": summary.resolved_state.cancelled_jobs_deleted,
            "expired_rate_limits_cleared": summary.resolved_state.expired_rate_limits_cleared,
            "media_files_deleted": summary.media.deleted,
            "media_bytes_reclaimed": summary.media.reclaimed_bytes,
        })),
    );

    Ok(summary)
}

#[tauri::command]
pub fn recover_stale_processing_jobs(
    database: State<'_, Database>,
    now: String,
    stale_before: String,
) -> Result<StaleJobRecoverySummary, String> {
    let summary = database
        .recover_stale_processing_jobs(&now, &stale_before)
        .map_err(|error| error.to_string())?;

    let _ = database.record_system_log(
        "warning",
        "Recovered stale processing jobs",
        Some(serde_json::json!({
            "requeued_jobs": summary.requeued_jobs,
            "stale_before": stale_before,
        })),
    );

    Ok(summary)
}

#[tauri::command]
pub fn retry_failed_account_import_jobs(
    database: State<'_, Database>,
    run_at: String,
) -> Result<Vec<JobSummary>, String> {
    let jobs = database
        .retry_failed_account_import_jobs(&run_at)
        .map_err(|error| error.to_string())?;

    let _ = database.record_system_log(
        "info",
        "Retried failed account imports",
        Some(serde_json::json!({
            "requeued_jobs": jobs.len(),
        })),
    );

    Ok(jobs)
}

#[tauri::command]
pub fn local_data_snapshot(database: State<'_, Database>) -> Result<LocalDataSnapshot, String> {
    database
        .local_data_snapshot()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn system_logs(database: State<'_, Database>) -> Result<Vec<SystemLogFile>, String> {
    database.system_logs().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn export_system_log(database: State<'_, Database>) -> Result<SystemLogExportSummary, String> {
    database
        .export_system_log()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_system_logs(database: State<'_, Database>) -> Result<SystemLogClearSummary, String> {
    database
        .clear_system_logs()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_local_backup(
    database: State<'_, Database>,
) -> Result<LocalBackupExportSummary, String> {
    let backup = database
        .export_local_backup()
        .map_err(|error| error.to_string())?;

    let _ = database.record_system_log(
        "info",
        "Created local backup",
        Some(serde_json::json!({
            "path": backup.path.clone(),
            "media_files": backup.media_files,
            "bytes": backup.bytes,
        })),
    );

    Ok(backup)
}

#[tauri::command]
pub fn restore_local_backup(
    database: State<'_, Database>,
    backup: LocalBackupRestoreForm,
) -> Result<LocalBackupRestoreSummary, String> {
    database
        .restore_local_backup(&backup)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn query_posts(
    database: State<'_, Database>,
    request: PostQueryRequest,
) -> Result<PostQueryResult, String> {
    database
        .query_posts(&request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn account_report(
    database: State<'_, Database>,
    request: ReportRequest,
) -> Result<ReportSnapshot, String> {
    database
        .account_report(&request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_metric(database: State<'_, Database>, metric: MetricForm) -> Result<bool, String> {
    database
        .save_metric(&metric)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_audience(
    database: State<'_, Database>,
    audience: AudienceForm,
) -> Result<bool, String> {
    database
        .save_audience(&audience)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_facebook_insight(
    database: State<'_, Database>,
    insight: FacebookInsightForm,
) -> Result<bool, String> {
    database
        .save_facebook_insight(&insight)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_account(database: State<'_, Database>, uuid: String) -> Result<bool, String> {
    database
        .delete_account(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn refresh_mastodon_account(
    database: State<'_, Database>,
    uuid: String,
) -> Result<AccountSummary, String> {
    database
        .refresh_mastodon_account(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn refresh_twitter_account(
    database: State<'_, Database>,
    uuid: String,
) -> Result<AccountSummary, String> {
    database
        .refresh_twitter_account(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn refresh_facebook_page_account(
    database: State<'_, Database>,
    uuid: String,
) -> Result<AccountSummary, String> {
    database
        .refresh_facebook_page_account(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn import_facebook_page_data(
    database: State<'_, Database>,
    uuid: String,
) -> Result<FacebookImportSummary, String> {
    database
        .import_facebook_page_data(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn import_twitter_account_data(
    database: State<'_, Database>,
    uuid: String,
) -> Result<TwitterImportSummary, String> {
    database
        .import_twitter_account_data(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn import_mastodon_account_data(
    database: State<'_, Database>,
    uuid: String,
) -> Result<MastodonImportSummary, String> {
    database
        .import_mastodon_account_data(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn queue_account_import(
    database: State<'_, Database>,
    uuid: String,
) -> Result<JobSummary, String> {
    let job = database
        .enqueue_account_import(&uuid, &Utc::now().to_rfc3339())
        .map_err(|error| error.to_string())?;

    let _ = database.record_system_log(
        "info",
        "Queued account import",
        Some(serde_json::json!({
            "account_uuid": uuid,
        })),
    );

    Ok(job)
}

#[tauri::command]
pub fn queue_all_account_imports(
    database: State<'_, Database>,
) -> Result<AccountImportQueueBatchSummary, String> {
    let summary = database
        .enqueue_all_account_imports(&Utc::now().to_rfc3339())
        .map_err(|error| error.to_string())?;

    let _ = database.record_system_log(
        "info",
        "Queued account import batch",
        Some(serde_json::json!({
            "requested_accounts": summary.requested_accounts,
            "eligible_accounts": summary.eligible_accounts,
            "queued_jobs": summary.queued_jobs,
            "skipped_unsupported": summary.skipped_unsupported,
            "skipped_unauthorized": summary.skipped_unauthorized,
        })),
    );

    Ok(summary)
}

#[tauri::command]
pub fn query_media_library(
    database: State<'_, Database>,
    request: MediaLibraryRequest,
) -> Result<Vec<MediaLibraryItem>, String> {
    database
        .query_media_library(&request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn import_media_file(
    database: State<'_, Database>,
    media: MediaImportForm,
) -> Result<MediaSummary, String> {
    database
        .import_media_file(&media)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn download_external_media(
    database: State<'_, Database>,
    media: MediaDownloadForm,
) -> Result<MediaSummary, String> {
    database
        .download_external_media(&media)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_media(database: State<'_, Database>, uuid: String) -> Result<bool, String> {
    database
        .delete_media(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn cleanup_orphaned_media_files(
    database: State<'_, Database>,
) -> Result<MediaCleanupSummary, String> {
    database
        .cleanup_orphaned_media_files()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn search_external_media(
    request: ExternalMediaSearchRequest,
) -> Result<ExternalMediaSearchResult, String> {
    search_external_media_provider(&request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_draft_post(
    database: State<'_, Database>,
    post: PostForm,
) -> Result<PostSummary, String> {
    database
        .create_draft_post(&post)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn post_detail(database: State<'_, Database>, uuid: String) -> Result<PostDetail, String> {
    database
        .post_detail(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn validate_post(
    database: State<'_, Database>,
    uuid: String,
) -> Result<PostValidationReport, String> {
    database
        .validate_post(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_post(
    database: State<'_, Database>,
    uuid: String,
    post: PostForm,
) -> Result<PostSummary, String> {
    database
        .update_post(&uuid, &post)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn duplicate_post(database: State<'_, Database>, uuid: String) -> Result<PostSummary, String> {
    database
        .duplicate_post(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_post(database: State<'_, Database>, uuid: String) -> Result<bool, String> {
    database
        .delete_post(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn bulk_delete_posts(
    database: State<'_, Database>,
    request: BulkDeletePostsForm,
) -> Result<BulkDeletePostsSummary, String> {
    database
        .bulk_delete_posts(&request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn schedule_post(
    database: State<'_, Database>,
    uuid: String,
    schedule: SchedulePostForm,
) -> Result<PostSummary, String> {
    database
        .schedule_post(&uuid, &schedule)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn retry_failed_post(
    database: State<'_, Database>,
    uuid: String,
    schedule: SchedulePostForm,
) -> Result<PostSummary, String> {
    database
        .retry_failed_post(&uuid, &schedule)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn settings(database: State<'_, Database>) -> Result<AppSettings, String> {
    database.settings().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_settings(
    database: State<'_, Database>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    settings.validate()?;
    database
        .save_settings(&settings)
        .map_err(|error| error.to_string())?;

    Ok(settings)
}

#[tauri::command]
pub fn services(database: State<'_, Database>) -> Result<Vec<ServiceSummary>, String> {
    database.services().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn service_credential_statuses(
    database: State<'_, Database>,
) -> Result<Vec<ServiceCredentialStatus>, String> {
    let services = database.services().map_err(|error| error.to_string())?;

    Ok(build_service_credential_statuses(&services))
}

#[tauri::command]
pub fn save_service_credential(
    database: State<'_, Database>,
    credential: ServiceCredentialForm,
) -> Result<Vec<ServiceCredentialStatus>, String> {
    let credential = credential.validated()?;
    let service_ref =
        save_service_credential_secret(&credential.service, &credential.field, &credential.value)
            .map_err(|error| error.to_string())?;

    database
        .save_service(&ServiceForm {
            name: credential.service,
            configuration_secret_ref: service_ref,
            configuration: None,
            active: true,
        })
        .map_err(|error| error.to_string())?;
    let services = database.services().map_err(|error| error.to_string())?;

    Ok(build_service_credential_statuses(&services))
}

#[tauri::command]
pub fn start_twitter_oauth(
    request: TwitterOAuthStartForm,
) -> Result<TwitterOAuthStartSummary, String> {
    start_twitter_oauth_provider(&request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn connect_twitter_account(
    database: State<'_, Database>,
    request: TwitterOAuthExchangeForm,
) -> Result<TwitterAccountConnection, String> {
    let authorization =
        connect_twitter_account_provider(&request).map_err(|error| error.to_string())?;
    let account = database
        .save_account(&authorization.account)
        .map_err(|error| error.to_string())?;

    Ok(TwitterAccountConnection { account })
}

#[tauri::command]
pub fn start_facebook_oauth(
    database: State<'_, Database>,
    mut request: FacebookOAuthStartForm,
) -> Result<FacebookOAuthStartSummary, String> {
    request.api_version = facebook_api_version(&database)?;

    start_facebook_oauth_provider(&request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn exchange_facebook_oauth(
    database: State<'_, Database>,
    mut request: FacebookOAuthExchangeForm,
) -> Result<FacebookUserConnectionSummary, String> {
    request.api_version = facebook_api_version(&database)?;
    let connection =
        exchange_facebook_oauth_provider(&request).map_err(|error| error.to_string())?;

    Ok(FacebookUserConnectionSummary {
        user_id: connection.user_id,
        user_name: connection.user_name,
        pages: connection.pages,
    })
}

#[tauri::command]
pub fn connect_facebook_pages(
    database: State<'_, Database>,
    mut request: FacebookPageConnectForm,
) -> Result<FacebookPageConnectionSummary, String> {
    request.api_version = facebook_api_version(&database)?;
    let connection =
        connect_facebook_pages_provider(&request).map_err(|error| error.to_string())?;
    let mut accounts = Vec::new();

    for account in connection.accounts {
        accounts.push(
            database
                .save_account(&account)
                .map_err(|error| error.to_string())?,
        );
    }

    Ok(FacebookPageConnectionSummary { accounts })
}

#[tauri::command]
pub fn register_mastodon_app(
    database: State<'_, Database>,
    request: MastodonAppRegistrationForm,
) -> Result<MastodonAppRegistrationSummary, String> {
    let registration =
        register_mastodon_app_provider(&request).map_err(|error| error.to_string())?;

    database
        .save_service(&ServiceForm {
            name: registration.service_name.clone(),
            configuration_secret_ref: format!("secret://services/{}", registration.service_name),
            configuration: None,
            active: true,
        })
        .map_err(|error| error.to_string())?;

    Ok(registration)
}

#[tauri::command]
pub fn connect_mastodon_account(
    database: State<'_, Database>,
    request: MastodonOAuthForm,
) -> Result<MastodonAccountConnection, String> {
    let authorization =
        connect_mastodon_account_provider(&request).map_err(|error| error.to_string())?;
    let account = database
        .save_account(&authorization.account)
        .map_err(|error| error.to_string())?;

    Ok(MastodonAccountConnection {
        server: authorization.server,
        account,
    })
}

#[tauri::command]
pub fn save_service(
    database: State<'_, Database>,
    service: ServiceForm,
) -> Result<ServiceSummary, String> {
    database
        .save_service(&service)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn tags(database: State<'_, Database>) -> Result<Vec<TagSummary>, String> {
    database.tags().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_tag(database: State<'_, Database>, tag: TagForm) -> Result<TagSummary, String> {
    database.create_tag(&tag).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_tag(
    database: State<'_, Database>,
    uuid: String,
    tag: TagForm,
) -> Result<TagSummary, String> {
    database
        .update_tag(&uuid, &tag)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_tag(database: State<'_, Database>, uuid: String) -> Result<bool, String> {
    database
        .delete_tag(&uuid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn run_due_jobs(
    database: State<'_, Database>,
    now: String,
    limit: Option<i64>,
) -> Result<WorkerRunSummary, String> {
    let summary = database
        .run_due_jobs(&now, limit.unwrap_or(10))
        .map_err(|error| error.to_string())?;

    let _ = database.record_system_log(
        "info",
        "Ran queued publishing work",
        Some(serde_json::json!({
            "reserved": summary.reserved,
            "completed": summary.completed,
            "failed": summary.failed,
            "deferred": summary.deferred
        })),
    );

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ServiceCredentialFieldStatus;

    #[derive(Debug)]
    struct RouteParity {
        route_name: &'static str,
        desktop_workflow: &'static str,
        desktop_coverage: &'static [&'static str],
    }

    const MIXPOST_ROUTE_PARITY: &[RouteParity] = &[
        RouteParity {
            route_name: "mixpost.dashboard",
            desktop_workflow: "Dashboard",
            desktop_coverage: &["dashboard_summary", "system_health"],
        },
        RouteParity {
            route_name: "mixpost.reports",
            desktop_workflow: "Reports",
            desktop_coverage: &["account_report"],
        },
        RouteParity {
            route_name: "mixpost.accounts.index",
            desktop_workflow: "Accounts",
            desktop_coverage: &["local_data_snapshot"],
        },
        RouteParity {
            route_name: "mixpost.accounts.add",
            desktop_workflow: "Accounts",
            desktop_coverage: &[
                "start_twitter_oauth",
                "connect_twitter_account",
                "start_facebook_oauth",
                "exchange_facebook_oauth",
                "register_mastodon_app",
                "connect_mastodon_account",
            ],
        },
        RouteParity {
            route_name: "mixpost.accounts.update",
            desktop_workflow: "Accounts",
            desktop_coverage: &[
                "refresh_twitter_account",
                "refresh_facebook_page_account",
                "refresh_mastodon_account",
            ],
        },
        RouteParity {
            route_name: "mixpost.accounts.delete",
            desktop_workflow: "Accounts",
            desktop_coverage: &["delete_account"],
        },
        RouteParity {
            route_name: "mixpost.accounts.entities.index",
            desktop_workflow: "Accounts",
            desktop_coverage: &["exchange_facebook_oauth"],
        },
        RouteParity {
            route_name: "mixpost.accounts.entities.store",
            desktop_workflow: "Accounts",
            desktop_coverage: &["connect_facebook_pages"],
        },
        RouteParity {
            route_name: "mixpost.posts.index",
            desktop_workflow: "Posts",
            desktop_coverage: &["query_posts"],
        },
        RouteParity {
            route_name: "mixpost.posts.create",
            desktop_workflow: "Posts",
            desktop_coverage: &["create_draft_post"],
        },
        RouteParity {
            route_name: "mixpost.posts.store",
            desktop_workflow: "Posts",
            desktop_coverage: &["create_draft_post"],
        },
        RouteParity {
            route_name: "mixpost.posts.edit",
            desktop_workflow: "Posts",
            desktop_coverage: &["post_detail"],
        },
        RouteParity {
            route_name: "mixpost.posts.update",
            desktop_workflow: "Posts",
            desktop_coverage: &["update_post"],
        },
        RouteParity {
            route_name: "mixpost.posts.delete",
            desktop_workflow: "Posts",
            desktop_coverage: &["delete_post"],
        },
        RouteParity {
            route_name: "mixpost.posts.schedule",
            desktop_workflow: "Posts",
            desktop_coverage: &["schedule_post", "retry_failed_post"],
        },
        RouteParity {
            route_name: "mixpost.posts.duplicate",
            desktop_workflow: "Posts",
            desktop_coverage: &["duplicate_post"],
        },
        RouteParity {
            route_name: "mixpost.posts.multipleDelete",
            desktop_workflow: "Posts",
            desktop_coverage: &["bulk_delete_posts"],
        },
        RouteParity {
            route_name: "mixpost.calendar",
            desktop_workflow: "Calendar",
            desktop_coverage: &["query_posts"],
        },
        RouteParity {
            route_name: "mixpost.media.index",
            desktop_workflow: "Media",
            desktop_coverage: &["query_media_library"],
        },
        RouteParity {
            route_name: "mixpost.media.delete",
            desktop_workflow: "Media",
            desktop_coverage: &["delete_media"],
        },
        RouteParity {
            route_name: "mixpost.media.fetchUploads",
            desktop_workflow: "Media",
            desktop_coverage: &["query_media_library"],
        },
        RouteParity {
            route_name: "mixpost.media.fetchStock",
            desktop_workflow: "Media",
            desktop_coverage: &["search_external_media"],
        },
        RouteParity {
            route_name: "mixpost.media.fetchGifs",
            desktop_workflow: "Media",
            desktop_coverage: &["search_external_media"],
        },
        RouteParity {
            route_name: "mixpost.media.download",
            desktop_workflow: "Media",
            desktop_coverage: &["download_external_media"],
        },
        RouteParity {
            route_name: "mixpost.media.upload",
            desktop_workflow: "Media",
            desktop_coverage: &["import_media_file"],
        },
        RouteParity {
            route_name: "mixpost.tags.store",
            desktop_workflow: "Tags",
            desktop_coverage: &["create_tag"],
        },
        RouteParity {
            route_name: "mixpost.tags.update",
            desktop_workflow: "Tags",
            desktop_coverage: &["update_tag"],
        },
        RouteParity {
            route_name: "mixpost.tags.delete",
            desktop_workflow: "Tags",
            desktop_coverage: &["delete_tag"],
        },
        RouteParity {
            route_name: "mixpost.settings.index",
            desktop_workflow: "Settings",
            desktop_coverage: &["settings"],
        },
        RouteParity {
            route_name: "mixpost.settings.update",
            desktop_workflow: "Settings",
            desktop_coverage: &["save_settings"],
        },
        RouteParity {
            route_name: "mixpost.services.index",
            desktop_workflow: "Services",
            desktop_coverage: &["services", "service_credential_statuses"],
        },
        RouteParity {
            route_name: "mixpost.services.update",
            desktop_workflow: "Services",
            desktop_coverage: &["save_service", "save_service_credential"],
        },
        RouteParity {
            route_name: "mixpost.services.createMastodonApp",
            desktop_workflow: "Accounts",
            desktop_coverage: &["register_mastodon_app"],
        },
        RouteParity {
            route_name: "mixpost.profile.index",
            desktop_workflow: "Profile",
            desktop_coverage: &["settings"],
        },
        RouteParity {
            route_name: "mixpost.profile.updateUser",
            desktop_workflow: "Profile",
            desktop_coverage: &["save_settings"],
        },
        RouteParity {
            route_name: "mixpost.profile.updatePassword",
            desktop_workflow: "Desktop replacement",
            desktop_coverage: &["not_applicable_local_desktop_profile"],
        },
        RouteParity {
            route_name: "mixpost.system.status",
            desktop_workflow: "System",
            desktop_coverage: &["system_health"],
        },
        RouteParity {
            route_name: "mixpost.system.logs.index",
            desktop_workflow: "System",
            desktop_coverage: &["system_logs"],
        },
        RouteParity {
            route_name: "mixpost.system.logs.download",
            desktop_workflow: "System",
            desktop_coverage: &["export_system_log"],
        },
        RouteParity {
            route_name: "mixpost.system.logs.clear",
            desktop_workflow: "System",
            desktop_coverage: &["clear_system_logs"],
        },
        RouteParity {
            route_name: "mixpost.refreshCsrfToken",
            desktop_workflow: "Desktop replacement",
            desktop_coverage: &["not_applicable_tauri_ipc"],
        },
        RouteParity {
            route_name: "mixpost.logout",
            desktop_workflow: "Desktop replacement",
            desktop_coverage: &["not_applicable_local_desktop_profile"],
        },
        RouteParity {
            route_name: "mixpost.callbackSocialProvider",
            desktop_workflow: "Accounts",
            desktop_coverage: &[
                "connect_twitter_account",
                "exchange_facebook_oauth",
                "connect_mastodon_account",
            ],
        },
    ];

    fn route_source_marker(route_name: &str) -> &'static str {
        match route_name {
            "mixpost.dashboard" => "DashboardController::class",
            "mixpost.reports" => "ReportsController::class",
            "mixpost.accounts.index" => "AccountsController::class, 'index'",
            "mixpost.accounts.add" => "AddAccountController::class",
            "mixpost.accounts.update" => "AccountsController::class, 'update'",
            "mixpost.accounts.delete" => "AccountsController::class, 'delete'",
            "mixpost.accounts.entities.index" => "AccountEntitiesController::class, 'index'",
            "mixpost.accounts.entities.store" => "AccountEntitiesController::class, 'store'",
            "mixpost.posts.index" => "PostsController::class, 'index'",
            "mixpost.posts.create" => "PostsController::class, 'create'",
            "mixpost.posts.store" => "PostsController::class, 'store'",
            "mixpost.posts.edit" => "PostsController::class, 'edit'",
            "mixpost.posts.update" => "PostsController::class, 'update'",
            "mixpost.posts.delete" => "PostsController::class, 'destroy'",
            "mixpost.posts.schedule" => "SchedulePostController::class",
            "mixpost.posts.duplicate" => "DuplicatePostController::class",
            "mixpost.posts.multipleDelete" => "DeletePostsController::class",
            "mixpost.calendar" => "CalendarController::class",
            "mixpost.media.index" => "MediaController::class, 'index'",
            "mixpost.media.delete" => "MediaController::class, 'destroy'",
            "mixpost.media.fetchUploads" => "MediaFetchUploadsController::class",
            "mixpost.media.fetchStock" => "MediaFetchStockController::class",
            "mixpost.media.fetchGifs" => "MediaFetchGifsController::class",
            "mixpost.media.download" => "MediaDownloadExternalController::class",
            "mixpost.media.upload" => "MediaUploadFileController::class",
            "mixpost.tags.store" => "TagsController::class, 'store'",
            "mixpost.tags.update" => "TagsController::class, 'update'",
            "mixpost.tags.delete" => "TagsController::class, 'destroy'",
            "mixpost.settings.index" => "SettingsController::class, 'index'",
            "mixpost.settings.update" => "SettingsController::class, 'update'",
            "mixpost.services.index" => "ServicesController::class, 'index'",
            "mixpost.services.update" => "ServicesController::class, 'update'",
            "mixpost.services.createMastodonApp" => "CreateMastodonAppController::class",
            "mixpost.profile.index" => "ProfileController::class, 'index'",
            "mixpost.profile.updateUser" => "UpdateAuthUserController::class",
            "mixpost.profile.updatePassword" => "UpdateAuthUserPasswordController::class",
            "mixpost.system.status" => "SystemStatusController::class",
            "mixpost.system.logs.index" => "SystemLogsController::class, 'index'",
            "mixpost.system.logs.download" => "SystemLogsController::class, 'download'",
            "mixpost.system.logs.clear" => "SystemLogsController::class, 'clear'",
            "mixpost.refreshCsrfToken" => "refresh-csrf-token",
            "mixpost.logout" => "AuthenticatedController::class",
            "mixpost.callbackSocialProvider" => "callback/{provider}",
            _ => panic!("missing source marker for {route_name}"),
        }
    }

    #[test]
    fn desktop_workflow_parity_covers_mixpost_routes() {
        let routes = include_str!("../../routes/web.php");

        for route in MIXPOST_ROUTE_PARITY {
            let marker = route_source_marker(route.route_name);

            assert!(
                routes.contains(marker),
                "{} must still exist in routes/web.php via marker {marker}",
                route.route_name,
            );
            assert!(
                !route.desktop_workflow.trim().is_empty(),
                "{} must name its desktop workflow",
                route.route_name
            );
            assert!(
                !route.desktop_coverage.is_empty(),
                "{} must have desktop command or explicit replacement coverage",
                route.route_name
            );
            assert!(
                route
                    .desktop_coverage
                    .iter()
                    .all(|item| !item.trim().is_empty()),
                "{} contains a blank desktop coverage item",
                route.route_name
            );
        }
    }

    #[test]
    fn builds_ok_system_health_without_issues() {
        let summary = build_system_health_summary(
            SystemHealthCounts {
                unauthorized_accounts: 0,
                failed_posts: 0,
                pending_jobs: 0,
                processing_jobs: 0,
                failed_jobs: 0,
                rate_limits: 0,
            },
            &[],
            "2026-06-24T15:00:00Z".to_string(),
        );

        assert_eq!(summary.status, "ok");
        assert!(summary.issues.is_empty());
        assert_eq!(summary.media_tools.ffmpeg.name, "FFmpeg");
        assert_eq!(summary.media_tools.ffprobe.name, "FFprobe");
    }

    #[test]
    fn builds_attention_system_health_for_blocking_issues() {
        let statuses = vec![ServiceCredentialStatus {
            service: "twitter".to_string(),
            label: "X/Twitter".to_string(),
            group: "social".to_string(),
            active: true,
            configured: false,
            fields: vec![ServiceCredentialFieldStatus {
                field: "client_secret".to_string(),
                label: "API Secret".to_string(),
                configured: false,
                env_vars: vec!["DUSTWAVE_TWITTER_CLIENT_SECRET".to_string()],
            }],
        }];
        let summary = build_system_health_summary(
            SystemHealthCounts {
                unauthorized_accounts: 1,
                failed_posts: 2,
                pending_jobs: 3,
                processing_jobs: 1,
                failed_jobs: 4,
                rate_limits: 5,
            },
            &statuses,
            "2026-06-24T15:00:00Z".to_string(),
        );

        assert_eq!(summary.status, "needs_attention");
        assert!(
            summary
                .issues
                .iter()
                .any(|issue| issue.title == "Unauthorized accounts")
        );
        assert!(
            summary
                .issues
                .iter()
                .any(|issue| issue.detail.contains("X/Twitter API Secret"))
        );
        assert!(
            summary
                .issues
                .iter()
                .any(|issue| issue.severity == "warning")
        );
        assert!(
            summary
                .issues
                .iter()
                .any(|issue| issue.title == "Processing background work")
        );
        assert!(!summary.media_tools.ffmpeg.command.trim().is_empty());
    }

    #[test]
    fn ignores_missing_credentials_for_inactive_services() {
        let statuses = vec![ServiceCredentialStatus {
            service: "klipy".to_string(),
            label: "Klipy".to_string(),
            group: "media".to_string(),
            active: false,
            configured: false,
            fields: vec![ServiceCredentialFieldStatus {
                field: "client_id".to_string(),
                label: "API Key".to_string(),
                configured: false,
                env_vars: vec!["DUSTWAVE_KLIPY_CLIENT_ID".to_string()],
            }],
        }];
        let summary = build_system_health_summary(
            SystemHealthCounts {
                unauthorized_accounts: 0,
                failed_posts: 0,
                pending_jobs: 0,
                processing_jobs: 0,
                failed_jobs: 0,
                rate_limits: 0,
            },
            &statuses,
            "2026-06-24T15:00:00Z".to_string(),
        );

        assert_eq!(summary.status, "ok");
        assert!(summary.issues.is_empty());
    }

    #[test]
    fn reports_missing_media_tool_without_failing_health() {
        let status = media_tool_status(
            "Test Tool",
            "DUST_WAVE_SOCIAL_TEST_MISSING_MEDIA_TOOL_PATH",
            "dust-wave-social-definitely-missing-media-tool",
        );

        assert_eq!(status.name, "Test Tool");
        assert!(!status.available);
        assert!(status.detail.contains("was not found"));
    }
}
