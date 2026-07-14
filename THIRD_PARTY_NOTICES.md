# Third-Party Notices

## FFmpeg And FFprobe

Dust Wave Social release builds that include bundled media sidecars use FFmpeg and FFprobe from the FFmpeg project.

Project policy: ship LGPL-only FFmpeg/FFprobe builds.

Release records to keep with each shipped build:

- FFmpeg version: `ffmpeg version 8.1.2 Copyright (c) 2000-2026 the FFmpeg developers`
- FFprobe version: `ffprobe version 8.1.2 Copyright (c) 2007-2026 the FFmpeg developers`
- Source archive: `https://ffmpeg.org/releases/ffmpeg-8.1.2.tar.xz`
- Source archive SHA-256: `464beb5e7bf0c311e68b45ae2f04e9cc2af88851abb4082231742a74d97b524c`
- Source signature: `https://ffmpeg.org/releases/ffmpeg-8.1.2.tar.xz.asc`
- Source signature key: `FCF986EA15E6E293A5644F10B4322F04D67658D8`, `FFmpeg release signing key <ffmpeg-devel@ffmpeg.org>`
- Build/configure flags: `--cc=clang --disable-shared --enable-static --disable-doc --disable-debug --disable-ffplay --disable-network --disable-autodetect --disable-gpl --disable-nonfree --disable-iconv --disable-audiotoolbox --disable-videotoolbox --disable-avfoundation --enable-small --extra-cflags='-mmacosx-version-min=11.0' --extra-ldflags='-mmacosx-version-min=11.0'`
- Binary source: locally built from the official FFmpeg 8.1.2 source archive on macOS 26.5.1 for `aarch64-apple-darwin`.
- FFmpeg binary SHA-256: `afaddb46af5d3053a13cf5d088c4cea4892c0916639b10587464ae49d2f13e4b`
- FFprobe binary SHA-256: `d02a39274d884c9c9a7dd23a66a12fb3fbe5b730b1a1217a1ed3a08dddff8e3a`
- Minimum macOS version: `11.0`
- License: GNU Lesser General Public License, as reported by `ffmpeg -L` and `ffprobe -L`.

The FFmpeg project is available at https://ffmpeg.org/. Its legal and license guidance is available at https://ffmpeg.org/legal.html.

Do not ship media sidecars built with `--enable-gpl`, `--enable-nonfree`, or GPL codec libraries such as x264/x265.
