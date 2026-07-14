# Media Sidecars

This directory is the staging point for optional FFmpeg/FFprobe sidecars used by Tauri release builds.

Tauri expects each source binary to include the Rust target triple:

- `ffmpeg-aarch64-apple-darwin`
- `ffprobe-aarch64-apple-darwin`
- `ffmpeg-x86_64-apple-darwin`
- `ffprobe-x86_64-apple-darwin`

Run `npm run desktop:media:prepare` after providing approved portable binaries. The script accepts:

- `DUSTWAVE_FFMPEG_BINARY=/absolute/path/to/ffmpeg`
- `DUSTWAVE_FFPROBE_BINARY=/absolute/path/to/ffprobe`
- `--target <rust-target-triple>` for cross-target staging

The staged binaries are ignored by git. Do not commit third-party FFmpeg binaries without confirming the selected build license and source-distribution obligations.
