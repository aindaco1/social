use std::env;
use std::ffi::OsStr;
#[cfg(debug_assertions)]
use std::path::Path;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaToolProbe {
    Available,
    Unavailable,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaToolSource {
    ConfiguredEnv,
    Bundled,
    SystemPath,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaToolResolution {
    pub command: String,
    pub source: MediaToolSource,
    pub probe: MediaToolProbe,
}

pub fn media_tool_command(env_var: &str, binary: &str) -> String {
    // Choosing a command for real work must not launch an extra version subprocess.
    select_media_tool(env_var, binary).0
}

pub fn resolve_media_tool(env_var: &str, binary: &str) -> MediaToolResolution {
    let (command, source) = select_media_tool(env_var, binary);
    let probe = probe_command(Command::new(&command).arg("-version"), PROBE_TIMEOUT);
    MediaToolResolution {
        command,
        source,
        probe,
    }
}

fn select_media_tool(env_var: &str, binary: &str) -> (String, MediaToolSource) {
    if let Ok(value) = env::var(env_var) {
        let command = value.trim().to_string();

        if !command.is_empty() {
            return (command, MediaToolSource::ConfiguredEnv);
        }
    }

    if let Some(command) = bundled_media_tool_path(binary) {
        let command = command.to_string_lossy().to_string();

        return (command, MediaToolSource::Bundled);
    }

    if let Some(command) = system_media_tool_path(binary) {
        let command = command.to_string_lossy().to_string();

        return (command, MediaToolSource::SystemPath);
    }

    (binary.to_string(), MediaToolSource::Path)
}

fn probe_command(command: &mut Command, timeout: Duration) -> MediaToolProbe {
    // Output is not needed: null streams avoid pipe backpressure and inherited-pipe hangs.
    let Ok(mut child) = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return MediaToolProbe::Unavailable;
    };
    wait_for_probe(&mut child, timeout)
}

fn wait_for_probe(child: &mut Child, timeout: Duration) -> MediaToolProbe {
    let started = Instant::now();
    let outcome = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    MediaToolProbe::Available
                } else {
                    MediaToolProbe::Unavailable
                };
            }
            Err(_) => break MediaToolProbe::Unavailable,
            Ok(None) => {}
        }
        if started.elapsed() >= timeout {
            break MediaToolProbe::TimedOut;
        }
        std::thread::sleep(
            Duration::from_millis(20).min(timeout.saturating_sub(started.elapsed())),
        );
    };
    // We own this child. Terminate and reap it rather than leaking one on every refresh.
    let _ = child.kill();
    let _ = child.wait();
    outcome
}

fn bundled_media_tool_path(binary: &str) -> Option<PathBuf> {
    bundled_media_tool_candidates(binary)
        .into_iter()
        .find(|candidate| candidate.exists())
}

fn bundled_media_tool_candidates(binary: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let binary_file_name = binary_file_name(binary);

    if let Ok(executable_path) = env::current_exe() {
        if let Some(executable_directory) = executable_path.parent() {
            candidates.push(executable_directory.join(&binary_file_name));

            if executable_directory
                .file_name()
                .is_some_and(|name| name == OsStr::new("MacOS"))
            {
                if let Some(contents_directory) = executable_directory.parent() {
                    candidates.push(contents_directory.join("Resources").join(&binary_file_name));
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    let staged_binary = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(format!(
            "{}-{}{}",
            binary,
            current_target_triple(),
            executable_extension()
        ));
    #[cfg(debug_assertions)]
    candidates.push(staged_binary);

    candidates
}

fn system_media_tool_path(binary: &str) -> Option<PathBuf> {
    [
        format!("/opt/homebrew/bin/{binary}"),
        format!("/usr/local/bin/{binary}"),
        format!("/usr/bin/{binary}"),
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|candidate| candidate.exists())
}

fn binary_file_name(binary: &str) -> String {
    format!("{binary}{}", executable_extension())
}

fn executable_extension() -> &'static str {
    if cfg!(windows) { ".exe" } else { "" }
}

pub fn current_target_triple() -> &'static str {
    option_env!("TAURI_ENV_TARGET_TRIPLE").unwrap_or_else(|| {
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            "aarch64-apple-darwin"
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            "x86_64-apple-darwin"
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            "x86_64-unknown-linux-gnu"
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            "aarch64-unknown-linux-gnu"
        } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            "x86_64-pc-windows-msvc"
        } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
            "aarch64-pc-windows-msvc"
        } else {
            "unknown"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_missing_tool_as_path_fallback() {
        let resolution = resolve_media_tool(
            "DUST_WAVE_SOCIAL_TEST_MISSING_MEDIA_TOOL_PATH",
            "dust-wave-social-definitely-missing-media-tool",
        );

        assert_eq!(
            resolution.command,
            "dust-wave-social-definitely-missing-media-tool"
        );
        assert_eq!(resolution.source, MediaToolSource::Path);
        assert_eq!(resolution.probe, MediaToolProbe::Unavailable);
    }

    #[cfg(unix)]
    #[test]
    fn probes_success_failure_and_large_output_without_pipes() {
        let timeout = Duration::from_secs(1);
        for (script, expected) in [
            ("exit 0", MediaToolProbe::Available),
            ("exit 7", MediaToolProbe::Unavailable),
            (
                "i=0; while [ $i -lt 20000 ]; do printf 'probe output\\n'; i=$((i+1)); done",
                MediaToolProbe::Available,
            ),
        ] {
            assert_eq!(
                probe_command(Command::new("/bin/sh").args(["-c", script]), timeout),
                expected
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn unresponsive_probe_times_out_and_reaps_its_child() {
        let mut child = Command::new("/bin/sleep").arg("60").spawn().unwrap();
        let started = Instant::now();
        assert_eq!(
            wait_for_probe(&mut child, Duration::from_millis(60)),
            MediaToolProbe::TimedOut
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(
            child.try_wait().unwrap().is_some(),
            "timed-out child must be reaped"
        );
    }

    #[test]
    fn exposes_target_triple_for_sidecar_staging() {
        assert!(!current_target_triple().trim().is_empty());
    }
}
