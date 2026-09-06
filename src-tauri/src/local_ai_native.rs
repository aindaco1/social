//! A fixed, bundled LiteRT helper. No caller-supplied executable or model paths.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use tauri::{Manager, State, WebviewWindow};

const INPUT_BYTES: usize = 128 * 128 * 3;
const OUTPUT_BYTES: usize = 512 * 512 * 3;
const MANIFEST: &str = include_str!("../../resources/desktop/public/litert/models/manifest.json");

#[derive(Default)]
pub struct NativeAiState(Mutex<Option<Arc<Session>>>);

#[derive(Default)]
struct Session {
    id: String,
    canceled: AtomicBool,
    child: Mutex<Option<Child>>,
    pipes: Mutex<Option<(ChildStdin, ChildStdout)>>,
}

impl Session {
    fn new(id: String) -> Self {
        Self {
            id,
            canceled: AtomicBool::new(false),
            child: Mutex::new(None),
            pipes: Mutex::new(None),
        }
    }
    fn stop(&self) {
        self.canceled.store(true, Ordering::SeqCst);
        if let Ok(mut child) = self.child.lock() {
            if let Some(mut child) = child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    fn check(&self) -> Result<(), String> {
        if self.canceled.load(Ordering::SeqCst) {
            Err("Local AI operation canceled".into())
        } else {
            Ok(())
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

impl NativeAiState {
    pub fn stop(&self) {
        if let Ok(mut active) = self.0.lock() {
            if let Some(session) = active.take() {
                session.stop();
            }
        }
    }

    fn session(&self, id: &str) -> Result<Arc<Session>, String> {
        self.0
            .lock()
            .map_err(|_| "Local AI session unavailable")?
            .as_ref()
            .filter(|session| session.id == id)
            .cloned()
            .ok_or_else(|| "Local AI session is no longer active".into())
    }
}

fn main_window(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("Local AI is available only in the main window".into())
    }
}

fn native_supported() -> bool {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return false;
    }
    // The pinned library's Mach-O minimum is 14, regardless of its wheel tag.
    Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|version| version.trim().split('.').next()?.parse::<u32>().ok())
        .is_some_and(|major| major >= 14)
}

fn bundled_paths(app: &tauri::AppHandle) -> Result<(PathBuf, PathBuf), String> {
    if cfg!(debug_assertions) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        return Ok((
            root.join("binaries/social-litert-aarch64-apple-darwin"),
            root.join("../resources/desktop/public/litert/models"),
        ));
    }
    let executable = std::env::current_exe().map_err(|_| "Local AI executable unavailable")?;
    let helper = executable
        .parent()
        .ok_or("Local AI bundle unavailable")?
        .join("social-litert");
    let models = app
        .path()
        .resource_dir()
        .map_err(|_| "Local AI resources unavailable")?
        .join("litert-models");
    Ok((helper, models))
}

fn verified_model(root: &std::path::Path) -> Result<PathBuf, String> {
    let manifest: serde_json::Value =
        serde_json::from_str(MANIFEST).map_err(|_| "Invalid bundled model manifest")?;
    let model = manifest["models"]
        .as_array()
        .and_then(|models| {
            models
                .iter()
                .find(|model| model["feature"] == "image_upscaling" && model["runtime"] == "litert")
        })
        .ok_or("Bundled upscaling model unavailable")?;
    let file = model["files"]
        .as_array()
        .and_then(|files| files.iter().find(|file| file["kind"] == "model"))
        .ok_or("Bundled upscaling weights unavailable")?;
    let relative = file["path"].as_str().ok_or("Invalid bundled model path")?;
    if std::path::Path::new(relative)
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err("Invalid bundled model path".into());
    }
    let path = root.join(relative);
    let bytes = std::fs::read(&path).map_err(|_| "Bundled model is missing; reinstall Social")?;
    if Some(bytes.len() as u64) != file["size"].as_u64()
        || format!("{:x}", Sha256::digest(&bytes)) != file["sha256"].as_str().unwrap_or("")
    {
        return Err("Bundled model failed its integrity check; reinstall Social".into());
    }
    Ok(path)
}

// Killing the child closes its pipes, interrupting a blocked read. The command
// never holds the child lock during inference, so Cancel does not wait for it.
fn bounded<T>(
    session: Arc<Session>,
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let (done, waiting) = mpsc::channel::<()>();
    let watchdog = session.clone();
    std::thread::spawn(move || {
        if matches!(
            waiting.recv_timeout(Duration::from_secs(60)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ) {
            watchdog.stop();
        }
    });
    let result = operation();
    let _ = done.send(());
    if result.is_err() {
        session.stop();
    }
    result
}

#[derive(Serialize)]
pub struct NativeReady {
    compile_ms: u32,
    cpu_threads: u32,
    runtime_version: &'static str,
}

#[tauri::command]
pub async fn local_ai_native_start(
    window: WebviewWindow,
    app: tauri::AppHandle,
    state: State<'_, NativeAiState>,
    database: State<'_, crate::db::Database>,
    session_id: String,
) -> Result<Option<NativeReady>, String> {
    main_window(&window)?;
    if uuid::Uuid::parse_str(&session_id).is_err() {
        return Err("Invalid Local AI session".into());
    }
    if !database
        .settings()
        .map_err(|_| "Local AI settings unavailable")?
        .local_ai_media_labs
    {
        return Err("Enable Local AI Media Labs in Settings first".into());
    }
    if !native_supported() {
        return Ok(None);
    }
    let (helper, models) = bundled_paths(&app)?;
    // Development checkouts may omit generated native assets. Packaged apps
    // must contain them; a broken package must not silently hide its failure.
    if cfg!(debug_assertions) && !helper.is_file() {
        return Ok(None);
    }
    let session = Arc::new(Session::new(session_id));
    {
        let mut active = state.0.lock().map_err(|_| "Local AI session unavailable")?;
        if active.is_some() {
            return Err("Another Local AI operation is already running".into());
        }
        *active = Some(session.clone());
    }
    tauri::async_runtime::spawn_blocking(move || {
        bounded(session.clone(), || {
            let model = verified_model(&models)?;
            session.check()?;
            let threads = std::thread::available_parallelism()
                .map(|cores| (cores.get() / 2).clamp(1, 4))
                .unwrap_or(1);
            {
                let mut child_slot = session
                    .child
                    .lock()
                    .map_err(|_| "Local AI process unavailable")?;
                session.check()?;
                let mut child = Command::new(helper)
                    .arg(model)
                    .arg(threads.to_string())
                    .env_clear()
                    .current_dir(models)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|_| "Bundled LiteRT could not start; reinstall Social or try again")?;
                let stdin = child.stdin.take().ok_or("Local AI input unavailable")?;
                let stdout = child.stdout.take().ok_or("Local AI output unavailable")?;
                *session
                    .pipes
                    .lock()
                    .map_err(|_| "Local AI pipes unavailable")? = Some((stdin, stdout));
                *child_slot = Some(child);
            }
            let mut pipes = session
                .pipes
                .lock()
                .map_err(|_| "Local AI pipes unavailable")?;
            let (_, stdout) = pipes.as_mut().ok_or("Local AI pipes unavailable")?;
            let mut ready = [0u8; 20];
            stdout
                .read_exact(&mut ready)
                .map_err(|_| "LiteRT model loading failed or timed out; retry the operation")?;
            let words: Vec<u32> = ready
                .chunks_exact(4)
                .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
                .collect();
            if words[0] != 0x3154524c
                || words[1] != INPUT_BYTES as u32
                || words[2] != OUTPUT_BYTES as u32
                || words[4] != threads as u32
            {
                return Err("Bundled LiteRT returned an incompatible model".into());
            }
            session.check()?;
            Ok(Some(NativeReady {
                compile_ms: words[3],
                cpu_threads: words[4],
                runtime_version: "2.1.6",
            }))
        })
    })
    .await
    .map_err(|_| "Local AI startup failed")?
}

#[tauri::command]
pub async fn local_ai_native_tile(
    window: WebviewWindow,
    state: State<'_, NativeAiState>,
    session_id: String,
    input: Vec<u8>,
) -> Result<tauri::ipc::Response, String> {
    main_window(&window)?;
    if input.len() != INPUT_BYTES {
        return Err("Invalid Local AI input tile".into());
    }
    let session = state.session(&session_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        bounded(session.clone(), || {
            session.check()?;
            let mut pipes = session
                .pipes
                .try_lock()
                .map_err(|_| "Local AI is already processing")?;
            let (stdin, stdout) = pipes.as_mut().ok_or("Local AI is not ready")?;
            stdin
                .write_all(b"T")
                .and_then(|_| stdin.write_all(&input))
                .and_then(|_| stdin.flush())
                .map_err(|_| "Local AI input failed; retry the operation")?;
            let mut output = vec![0; OUTPUT_BYTES + 4];
            stdout
                .read_exact(&mut output)
                .map_err(|_| "LiteRT inference failed or timed out; retry the operation")?;
            session.check()?;
            Ok(tauri::ipc::Response::new(output))
        })
    })
    .await
    .map_err(|_| "Local AI inference failed")?
}

#[tauri::command]
pub fn local_ai_native_close(
    window: WebviewWindow,
    state: State<'_, NativeAiState>,
    session_id: String,
) -> Result<(), String> {
    main_window(&window)?;
    let mut active = state.0.lock().map_err(|_| "Local AI session unavailable")?;
    if active
        .as_ref()
        .is_some_and(|session| session.id == session_id)
    {
        if let Some(session) = active.take() {
            session.stop();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_model_matches_canonical_manifest() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources/desktop/public/litert/models");
        assert!(verified_model(&root).unwrap().is_file());
        assert!(verified_model(&root.join("missing")).is_err());
    }
    #[test]
    fn stopping_is_idempotent_and_closes_only_matching_session() {
        let session = Arc::new(Session::new("test".into()));
        let state = NativeAiState(Mutex::new(Some(session.clone())));
        assert!(state.session("other").is_err());
        assert!(state.session("test").is_ok());
        state.stop();
        state.stop();
        assert!(session.check().is_err());
        assert!(state.session("test").is_err());
    }
}
