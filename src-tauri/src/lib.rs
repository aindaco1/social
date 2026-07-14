mod commands;
mod db;
mod domain;
mod external_media;
mod facebook;
mod mastodon;
mod media_tools;
mod secrets;
mod twitter;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            let database = db::Database::initialize(app.handle())?;
            app.manage(database);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system_health,
            commands::dashboard_summary,
            commands::app_data_directory,
            commands::clear_resolved_system_state,
            commands::run_desktop_maintenance,
            commands::recover_stale_processing_jobs,
            commands::retry_failed_account_import_jobs,
            commands::local_data_snapshot,
            commands::system_logs,
            commands::export_system_log,
            commands::clear_system_logs,
            commands::create_local_backup,
            commands::restore_local_backup,
            commands::query_posts,
            commands::account_report,
            commands::save_metric,
            commands::save_audience,
            commands::save_facebook_insight,
            commands::delete_account,
            commands::refresh_mastodon_account,
            commands::refresh_twitter_account,
            commands::refresh_facebook_page_account,
            commands::import_mastodon_account_data,
            commands::import_twitter_account_data,
            commands::import_facebook_page_data,
            commands::queue_account_import,
            commands::queue_all_account_imports,
            commands::query_media_library,
            commands::import_media_file,
            commands::download_external_media,
            commands::delete_media,
            commands::cleanup_orphaned_media_files,
            commands::search_external_media,
            commands::create_draft_post,
            commands::post_detail,
            commands::validate_post,
            commands::update_post,
            commands::duplicate_post,
            commands::delete_post,
            commands::bulk_delete_posts,
            commands::schedule_post,
            commands::retry_failed_post,
            commands::settings,
            commands::save_settings,
            commands::services,
            commands::service_credential_statuses,
            commands::save_service_credential,
            commands::start_twitter_oauth,
            commands::connect_twitter_account,
            commands::start_facebook_oauth,
            commands::exchange_facebook_oauth,
            commands::connect_facebook_pages,
            commands::register_mastodon_app,
            commands::connect_mastodon_account,
            commands::save_service,
            commands::tags,
            commands::create_tag,
            commands::update_tag,
            commands::delete_tag,
            commands::run_due_jobs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dust Wave Social");
}
