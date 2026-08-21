use serde::{Deserialize, Serialize};
use std::{env, fs, process, thread, time::Duration};
use tauri::App;
use tauri_plugin_updater::UpdaterExt;

#[derive(Clone, Deserialize, Serialize)]
struct SmokeReport {
    ok: bool,
    kind: String,
    mode: String,
    package_version: String,
    running_version: String,
    expected_version: Option<String>,
    found_update: bool,
    update_version: Option<String>,
    downloaded_bytes: Option<usize>,
    install_started: bool,
    restart_requested: bool,
    restarted: bool,
    source_pid: Option<u32>,
    final_pid: Option<u32>,
    error: Option<String>,
}

pub fn maybe_spawn(app: &App) {
    if env::var_os("DUSTWAVE_UPDATER_SMOKE").is_some() {
        spawn_updater_smoke(app);
    } else if matches!(env::var("DUSTWAVE_DESKTOP_SMOKE").as_deref(), Ok("launch")) {
        spawn_launch_smoke(app);
    }
}

fn spawn_launch_smoke(app: &App) {
    let package_version = app.package_info().version.to_string();
    let delay_ms = env::var("DUSTWAVE_DESKTOP_SMOKE_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1500);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(delay_ms));
        finish(
            SmokeReport {
                ok: true,
                kind: "launch".to_string(),
                mode: "launch".to_string(),
                package_version: package_version.clone(),
                running_version: package_version,
                expected_version: None,
                found_update: false,
                update_version: None,
                downloaded_bytes: None,
                install_started: false,
                restart_requested: false,
                restarted: false,
                source_pid: None,
                final_pid: Some(process::id()),
                error: None,
            },
            0,
        );
    });
}

fn spawn_updater_smoke(app: &App) {
    let handle = app.handle().clone();
    let package_version = app.package_info().version.to_string();
    let mode = env::var("DUSTWAVE_UPDATER_SMOKE").unwrap_or_else(|_| "download".to_string());
    let expected_version = env::var("DUSTWAVE_UPDATER_EXPECT_VERSION").ok();
    let force_update = env::var_os("DUSTWAVE_UPDATER_SMOKE_FORCE_UPDATE").is_some();

    tauri::async_runtime::spawn(async move {
        let result = run_updater_smoke(
            handle,
            package_version.clone(),
            mode.clone(),
            expected_version.clone(),
            force_update,
        )
        .await;

        match result {
            Ok(report) if report.restart_requested && !report.restarted => {
                // The report was written before the restart request. The replacement process
                // will complete the hop report and exit through this same smoke entry point.
            }
            Ok(report) => finish(report, 0),
            Err(error) => finish(
                SmokeReport {
                    ok: false,
                    kind: "updater".to_string(),
                    mode,
                    package_version: package_version.clone(),
                    running_version: package_version,
                    expected_version,
                    found_update: false,
                    update_version: None,
                    downloaded_bytes: None,
                    install_started: false,
                    restart_requested: false,
                    restarted: false,
                    source_pid: None,
                    final_pid: Some(process::id()),
                    error: Some(error),
                },
                1,
            ),
        }
    });
}

async fn run_updater_smoke(
    handle: tauri::AppHandle,
    package_version: String,
    mode: String,
    expected_version: Option<String>,
    force_update: bool,
) -> Result<SmokeReport, String> {
    if mode == "hop" && expected_version.as_deref() == Some(package_version.as_str()) {
        return complete_updater_hop(package_version, expected_version);
    }

    let mut builder = handle.updater_builder().timeout(Duration::from_secs(120));

    if force_update {
        builder = builder.version_comparator(|_current, _remote| true);
    }

    let updater = builder.build().map_err(|error| error.to_string())?;
    let update = updater.check().await.map_err(|error| error.to_string())?;
    let Some(update) = update else {
        return Err("expected an update, but updater reported no update".to_string());
    };

    if let Some(expected) = expected_version.as_deref() {
        if update.version != expected {
            return Err(format!(
                "updater reported version {}, expected {}",
                update.version, expected
            ));
        }
    }

    let bytes = update
        .download(|_, _| {}, || {})
        .await
        .map_err(|error| error.to_string())?;

    if bytes.is_empty() {
        return Err("updater downloaded an empty package".to_string());
    }

    let install_mode = matches!(mode.as_str(), "install" | "download-and-install" | "hop");
    let mut report = SmokeReport {
        ok: true,
        kind: "updater".to_string(),
        mode,
        package_version: package_version.clone(),
        running_version: package_version,
        expected_version,
        found_update: true,
        update_version: Some(update.version.clone()),
        downloaded_bytes: Some(bytes.len()),
        install_started: install_mode,
        restart_requested: false,
        restarted: false,
        source_pid: install_mode.then(process::id),
        final_pid: None,
        error: None,
    };

    if install_mode {
        emit_report(&report);
        update.install(bytes).map_err(|error| error.to_string())?;

        if report.mode == "hop" {
            report.restart_requested = true;
            emit_report(&report);
            handle.request_restart();
        }
    }

    Ok(report)
}

fn complete_updater_hop(
    running_version: String,
    expected_version: Option<String>,
) -> Result<SmokeReport, String> {
    let report_path = smoke_report_path()
        .ok_or_else(|| "updater hop requires a smoke report path".to_string())?;
    let prior_payload = fs::read_to_string(&report_path)
        .map_err(|error| format!("unable to read updater hop report: {error}"))?;
    let prior: SmokeReport = serde_json::from_str(&prior_payload)
        .map_err(|error| format!("invalid updater hop report: {error}"))?;
    let source_pid = prior
        .source_pid
        .ok_or_else(|| "updater hop report is missing the source PID".to_string())?;

    if prior.mode != "hop" || !prior.install_started || !prior.restart_requested {
        return Err(
            "updater hop did not record a completed install and restart request".to_string(),
        );
    }

    if source_pid == process::id() {
        return Err("updater hop relaunched with the original process ID".to_string());
    }

    Ok(SmokeReport {
        ok: true,
        kind: "updater".to_string(),
        mode: "hop".to_string(),
        package_version: prior.package_version,
        running_version,
        expected_version,
        found_update: prior.found_update,
        update_version: prior.update_version,
        downloaded_bytes: prior.downloaded_bytes,
        install_started: true,
        restart_requested: true,
        restarted: true,
        source_pid: Some(source_pid),
        final_pid: Some(process::id()),
        error: None,
    })
}

fn smoke_report_path() -> Option<String> {
    [
        "DUSTWAVE_SMOKE_REPORT",
        "DUSTWAVE_DESKTOP_SMOKE_REPORT",
        "DUSTWAVE_UPDATER_SMOKE_REPORT",
    ]
    .into_iter()
    .find_map(|name| env::var(name).ok())
}

fn emit_report(report: &SmokeReport) {
    let payload = serde_json::to_string_pretty(report)
        .unwrap_or_else(|error| format!("{{\"ok\":false,\"error\":\"{error}\"}}"));

    if let Ok(path) = env::var("DUSTWAVE_SMOKE_REPORT") {
        let _ = fs::write(path, format!("{payload}\n"));
    }

    if let Ok(path) = env::var("DUSTWAVE_DESKTOP_SMOKE_REPORT") {
        let _ = fs::write(path, format!("{payload}\n"));
    }

    if let Ok(path) = env::var("DUSTWAVE_UPDATER_SMOKE_REPORT") {
        let _ = fs::write(path, format!("{payload}\n"));
    }
}

fn finish(report: SmokeReport, code: i32) -> ! {
    emit_report(&report);

    if code == 0 {
        println!(
            "DUSTWAVE_SMOKE_REPORT {}",
            serde_json::to_string(&report).unwrap_or_default()
        );
    } else {
        eprintln!(
            "DUSTWAVE_SMOKE_REPORT {}",
            serde_json::to_string(&report).unwrap_or_default()
        );
    }

    process::exit(code);
}
