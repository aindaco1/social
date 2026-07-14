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

For the Apple Silicon MVP release path, run `npm run desktop:media:build-lgpl` to build FFmpeg/FFprobe from the official FFmpeg source archive, verify the recorded source SHA-256, and stage LGPL-only sidecars through the same validation script. GitHub Actions runs this source-build step when the Desktop workflow input `build_media_sidecars` is `true`.

The staged binaries are ignored by git. Do not commit third-party FFmpeg binaries without confirming the selected build license and source-distribution obligations.
