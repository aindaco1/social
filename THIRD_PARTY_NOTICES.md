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

## LiteRT.js

Dust Wave Social release builds include the LiteRT.js browser runtime from the `@litertjs/core` package for local AI media Labs capability probing and on-device model execution.

- Package: `@litertjs/core`
- Version: `2.5.2`
- License: Apache License 2.0
- Source: `https://github.com/google-ai-edge/LiteRT/tree/main/litert/js/packages/core`
- Bundled runtime assets: `resources/desktop/public/litert/wasm/*.js` and `resources/desktop/public/litert/wasm/*.wasm`

## Real-ESRGAN-x4plus Model

Dust Wave Social release builds include a quantized TFLite Real-ESRGAN-x4plus model for local-only image upscaling Labs work. The model bundle is validated by `npm run local-ai:models:check`.

- Model: `Real-ESRGAN-x4plus`
- Use: local image upscaling behind the Local AI Media Labs flag
- Runtime: LiteRT.js
- Format: TFLite
- Precision: `w8a8`
- Version/source package: `qai-hub-models-v0.57.3`
- Source package URL: `https://qaihub-public-assets.s3.us-west-2.amazonaws.com/qai-hub-models/models/real_esrgan_x4plus/releases/v0.57.3/real_esrgan_x4plus-tflite-w8a8.zip`
- Source package SHA-256: `cfaec8e3491ee7ebdaaf3665bab6f8ed75cc2adbeb3c0f6db343df512440fceb`
- Model file: `resources/desktop/public/litert/models/real-esrgan-x4plus-w8a8/real_esrgan_x4plus.tflite`
- Model file SHA-256: `1b40d1e68931fa6bd599e7ebe62c536cf02cb888c3a49a817ee646c2906e326a`
- Vendor metadata SHA-256: `9261d3f66ccdaada22114054cd3894e8e567e05ca5a0724fb87fbb8a4aaa32f0`
- License notice SHA-256: `4a699ec4863d96a91fc265948a0c90033f7e8735d515524dcf3444736406e0c2`
- License: BSD-3-Clause, recorded in `resources/desktop/public/litert/models/real-esrgan-x4plus-w8a8/LICENSE.BSD-3-Clause.txt`
- Vendor model page: `https://aihub.qualcomm.com/models/real_esrgan_x4plus`
- Model card: `https://huggingface.co/qualcomm/Real-ESRGAN-x4plus`

Future `.tflite`, ONNX, MediaPipe, or other model files must have their license, source, version, intended local-only use, redistribution permission, file size, and SHA-256 recorded before release.
