use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    pub available: bool,
}

pub fn media_tool_command(env_var: &str, binary: &str) -> String {
    resolve_media_tool(env_var, binary).command
}

pub fn resolve_media_tool(env_var: &str, binary: &str) -> MediaToolResolution {
    if let Ok(value) = env::var(env_var) {
        let command = value.trim().to_string();

        if !command.is_empty() {
            return MediaToolResolution {
                available: command_is_available(&command),
                command,
                source: MediaToolSource::ConfiguredEnv,
            };
        }
    }

    if let Some(command) = bundled_media_tool_path(binary) {
        let command = command.to_string_lossy().to_string();

        return MediaToolResolution {
            available: command_is_available(&command),
            command,
            source: MediaToolSource::Bundled,
        };
    }

    if let Some(command) = system_media_tool_path(binary) {
        let command = command.to_string_lossy().to_string();

        return MediaToolResolution {
            available: command_is_available(&command),
            command,
            source: MediaToolSource::SystemPath,
        };
    }

    MediaToolResolution {
        available: command_is_available(binary),
        command: binary.to_string(),
        source: MediaToolSource::Path,
    }
}

fn command_is_available(command: &str) -> bool {
    Command::new(command)
        .arg("-version")
        .output()
        .is_ok_and(|output| output.status.success())
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

    let staged_binary = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(format!(
            "{}-{}{}",
            binary,
            current_target_triple(),
            executable_extension()
        ));
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
        assert!(!resolution.available);
    }

    #[test]
    fn exposes_target_triple_for_sidecar_staging() {
        assert!(!current_target_triple().trim().is_empty());
    }
}
