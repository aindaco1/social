use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, ipc::Channel};
use tauri_plugin_updater::UpdaterExt;

const UPDATE_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum SoftwareUpdateEvent {
    Started {
        #[serde(rename = "contentLength")]
        content_length: Option<u64>,
    },
    Progress {
        #[serde(rename = "chunkLength")]
        chunk_length: usize,
    },
    Finished,
    Installing,
    Restarting,
}

fn validate_update_version(found: Option<&str>, expected: &str) -> Result<(), String> {
    if expected.trim().is_empty() {
        return Err("Expected update version is missing".to_string());
    }

    let found = found.ok_or_else(|| "The selected update is no longer available".to_string())?;

    if found != expected {
        return Err(format!(
            "The available update changed from {expected} to {found}; check again before installing"
        ));
    }

    Ok(())
}

#[tauri::command]
pub async fn install_software_update_and_restart(
    app: AppHandle,
    expected_version: String,
    on_event: Channel<SoftwareUpdateEvent>,
) -> Result<(), String> {
    let expected_version = expected_version.trim().to_string();
    validate_update_version(Some(expected_version.as_str()), expected_version.as_str())?;

    let updater = app
        .updater_builder()
        .timeout(UPDATE_TIMEOUT)
        .build()
        .map_err(|error| error.to_string())?;
    let update = updater.check().await.map_err(|error| error.to_string())?;

    validate_update_version(
        update.as_ref().map(|available| available.version.as_str()),
        expected_version.as_str(),
    )?;

    let update = update.expect("validated available update");
    let download_channel = on_event.clone();
    let finished_channel = on_event.clone();
    let mut first_chunk = true;
    let bytes = update
        .download(
            move |chunk_length, content_length| {
                if first_chunk {
                    first_chunk = false;
                    let _ = download_channel.send(SoftwareUpdateEvent::Started { content_length });
                }

                let _ = download_channel.send(SoftwareUpdateEvent::Progress { chunk_length });
            },
            move || {
                let _ = finished_channel.send(SoftwareUpdateEvent::Finished);
            },
        )
        .await
        .map_err(|error| error.to_string())?;

    let _ = on_event.send(SoftwareUpdateEvent::Installing);
    update.install(bytes).map_err(|error| error.to_string())?;
    let _ = on_event.send(SoftwareUpdateEvent::Restarting);

    // Request the restart from the Rust side before replying to the webview. On macOS the
    // updater moves the running bundle during installation, and a JavaScript invoke can remain
    // pending after that swap. Keeping installation and restart in one backend command ensures
    // the old process cannot be left displaying a permanent "Installing update" state.
    app.request_restart();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{SoftwareUpdateEvent, validate_update_version};

    #[test]
    fn update_version_guard_accepts_only_the_selected_release() {
        assert!(validate_update_version(Some("0.1.5"), "0.1.5").is_ok());
        assert!(validate_update_version(None, "0.1.5").is_err());
        assert!(validate_update_version(Some("0.1.6"), "0.1.5").is_err());
        assert!(validate_update_version(Some("0.1.5"), "").is_err());
    }

    #[test]
    fn update_progress_events_keep_the_frontend_channel_contract() {
        assert_eq!(
            serde_json::to_value(SoftwareUpdateEvent::Started {
                content_length: Some(42)
            })
            .unwrap(),
            serde_json::json!({
                "event": "Started",
                "data": { "contentLength": 42 }
            })
        );
        assert_eq!(
            serde_json::to_value(SoftwareUpdateEvent::Progress { chunk_length: 7 }).unwrap(),
            serde_json::json!({
                "event": "Progress",
                "data": { "chunkLength": 7 }
            })
        );
        assert_eq!(
            serde_json::to_value(SoftwareUpdateEvent::Restarting).unwrap(),
            serde_json::json!({ "event": "Restarting" })
        );
    }
}
