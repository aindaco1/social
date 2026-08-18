# LiteRT.js And Local AI Media MVP Plan

Updated: 2026-07-15

Audience: Dust Wave product, engineering, and release owners evaluating local AI media features.

## Decision

Local AI media tooling is now MVP scope, behind a Settings or Labs flag. The MVP implementation should use LiteRT.js as the primary runtime for local image upscaling and compatible computer-vision models, with all inference running on device inside the packaged Tauri app.

Do not add cloud inference, remote model calls, or hidden provider fallbacks for MVP. If a feature cannot run locally with packaged runtime assets and reviewed model weights, it is not MVP-ready.

## Implementation Status

Implemented:

- Settings-controlled Local AI Media Labs flag.
- Bundled `@litertjs/core` Wasm runtime assets in the desktop public bundle.
- Runtime probe for LiteRT Wasm and WebGPU availability.
- Bundled Real-ESRGAN-x4plus `w8a8` TFLite model, vendor metadata, BSD-3-Clause notice, and checksum manifest.
- Release asset check for bundled LiteRT Wasm files and local AI model files.
- Labs runtime probe that loads LiteRT Wasm, reads the model manifest, and compiles the bundled upscaling model.
- Model-backed tiled x4 image upscaling through LiteRT.js, with WebGPU detection, Wasm fallback, progress, cancellation between tiles, and original-preserving derivative media records.
- Deterministic local upscaling remains available as a fallback command, and deterministic crop suggestions save derivative media.
- Local AI derivative metadata records source media, model/runtime details, source/output dimensions, source/output SHA-256 hashes, operation type, and creation time.
- Deterministic media quality preflight.
- Local metadata-backed profile media search using filename, MIME type, media type, dimensions, orientation, broad color tone, brightness, and file size signals.
- Editable deterministic profile-backed alt-text drafting that remains review-required and avoids object/person claims.

Still implementation/acceptance-gated:

- True shared image/text embeddings for richer semantic search.
- Model-backed image captioning for richer alt-text drafts.
- Packaged offline acceptance for selected model behavior in the signed/stapled app.

## Alternative Review

No single alternative is clearly better than LiteRT.js for the whole Dust Wave use case.

- LiteRT.js: best aligned with local image upscaling because Google's launch material specifically demonstrates Real-ESRGAN-style 4x browser upscaling with tiled patches. It also gives Dust Wave a modern `.tflite` path with Wasm, WebGPU, and emerging WebNN acceleration.
- ONNX Runtime Web: more mature and model-rich for broad browser inference. It supports browser-side Wasm, WebGL, WebGPU, and WebNN execution providers, and ONNX models are widely available. It is the strongest fallback candidate if an MVP model is only practically available in ONNX.
- Transformers.js: strongest for semantic media search and alt-text drafting because it exposes common pretrained model pipelines and runs through ONNX Runtime in the browser, including WebGPU for supported models. It is a good adapter candidate if LiteRT-compatible captioning or embedding models do not pass acceptance.
- MediaPipe Tasks Web: strong for image/text embeddings, object detection, and lightweight CV tasks. It is a good adapter candidate for semantic search or preflight if its packaged models and licensing are cleaner than a LiteRT alternative.
- TensorFlow.js: mature and flexible, but less attractive as the primary new runtime because LiteRT.js is the newer Google AI Edge direction for `.tflite` web inference, and ONNX/Transformers currently offer broader model coverage for non-upscaling tasks.
- Native Core ML or MLX: attractive for Apple Silicon performance, but not the first MVP path because it would add a macOS-specific native model pipeline and more packaging/signing surface. Revisit after the Tauri/WebView local path is proven.

MVP architecture: use LiteRT.js first where viable, but keep the local AI service behind a small runtime adapter interface so a specific feature can use ONNX Runtime Web, Transformers.js, or MediaPipe only when LiteRT.js cannot meet model, quality, license, or packaged-runtime requirements. Any adapter must still be local-only, packaged, tested offline, and documented.

Relevant sources:

- Google Developers Blog: `https://developers.googleblog.com/litertjs-googles-high-performance-web-ai-inference/`
- LiteRT.js docs: `https://developers.google.com/edge/litert/web`
- ONNX Runtime Web docs: `https://onnxruntime.ai/docs/tutorials/web/`
- Transformers.js docs: `https://huggingface.co/docs/transformers.js/en/index`
- MediaPipe Solutions docs: `https://developers.google.com/edge/mediapipe/solutions/guide`
- MediaPipe Image Embedder for Web: `https://developers.google.com/edge/mediapipe/solutions/vision/image_embedder/web_js`
- TensorFlow.js docs: `https://www.tensorflow.org/js`
- web.dev client-side AI stack: `https://web.dev/learn/ai/client-side`
- Apple Safari 26 release notes for WebGPU support: `https://developer.apple.com/documentation/safari-release-notes/safari-26-release-notes`

## MVP Feature Scope

### Local Image Upscaling

Use case: an operator imports a low-resolution image, chooses Upscale locally, reviews the output, and saves the enhanced version as a separate derivative media item.

Requirements:

- Bundle `@litertjs/core` Wasm locally; do not depend on a CDN at runtime.
- Bundle only model files with approved redistribution rights.
- Prefer a `.tflite` upscaling model that runs through LiteRT.js. Use ONNX only if the selected model cannot be converted or run acceptably through LiteRT.js.
- Preserve the original media and save the enhanced output as a derivative, not an overwrite.
- Record model name, model version, model license, runtime, scale factor, source media ID, source file hash, output file hash, and creation time.
- Show progress, cancellation, and failure states for large images.
- Feature-detect WebGPU and fall back to Wasm or disable the feature with a clear message.
- Tile large images to control memory use.
- Test in the signed/stapled packaged Tauri app, not only in Vite dev mode.

### Media Quality Preflight

Use case: before publishing, the app flags media that may fail provider rules or look poor in a post.

Requirements:

- Run local checks for dimensions, aspect ratio, file size, MIME type, duration, frame rate where available, transparency, likely blur, likely compression damage, and unreadable thumbnail risk.
- Prefer deterministic image/video metadata and signal-processing checks before ML.
- Use local ML only when it adds materially better detection than simple metadata checks.
- Store preflight results as advisory metadata; never mutate source files automatically.
- Make every warning provider-specific when possible, especially for Instagram, Facebook Pages, X, Mastodon, and TikTok-assisted workflows.

### Smart Crop Suggestions

Use case: the app suggests safe crops for provider ratios while the operator keeps final control.

Requirements:

- Generate crop suggestions for common provider formats such as square, portrait, landscape, story/reel-style vertical, and preview thumbnails.
- Preserve the original media.
- Save accepted crops as derivative media with crop rectangle, target provider/ratio, source media ID, and model/runtime metadata if ML was used.
- Avoid auto-publishing a suggested crop without operator review.
- Prefer deterministic saliency/face/object region heuristics for MVP. Use ML only as a review aid.

### Local Semantic Media Search

Use case: the operator searches the local media library by meaning, not only filename or tags.

Requirements:

- Generate local embeddings for imported media and operator-provided text queries.
- Store embeddings locally in app data, excluded from support exports unless explicitly included.
- Do not send media, thumbnails, embeddings, or search terms to a third-party service.
- Record embedding model name, model version, license, dimension, source media ID, and creation time.
- Provide reindex and delete behavior when media is removed.
- Use LiteRT.js if a suitable shared image/text embedding model passes acceptance. Use MediaPipe or Transformers.js only if they provide a better licensed local model path.

### Alt-Text Drafting

Use case: the app drafts accessible alt text for imported images, and the operator edits or approves it before publishing.

Requirements:

- Treat generated alt text as a draft only.
- Require visible operator review before attaching it to a publish target.
- Preserve user-edited alt text separately from generated suggestions.
- Store model name, model version, license, runtime, source media ID, and creation time for generated suggestions.
- Avoid identity, demographic, medical, legal, or sensitive-attribute claims unless the operator writes them manually.
- Prefer concise descriptive captions over engagement-oriented copy.
- Use LiteRT.js if a suitable local image-captioning or vision-language model passes acceptance. Use Transformers.js only if it is the practical local model path and its model license permits bundled desktop distribution.

## Packaging Requirements

- All MVP runtime assets must be bundled with the app or staged by a documented offline installer step before first use.
- No CDN runtime loading.
- No runtime model download during normal MVP use.
- No hidden cloud fallback.
- No secrets, access tokens, embeddings, media hashes, generated captions, or enhanced images in setup packets or support exports unless explicitly selected by the operator.
- Include third-party runtime and model notices in `THIRD_PARTY_NOTICES.md`.
- Keep release artifact checks for bundled Wasm/model files, metadata, notices, file sizes, and SHA-256 checksums.

## UI Requirements

- Gate the feature under a Settings or Labs flag named clearly enough that operators know local AI processing is involved.
- Show whether WebGPU, Wasm fallback, or disabled mode is active.
- Show progress, cancellation, retry, and clear failure messages.
- Keep original and derivative media visually distinct.
- Never make generated alt text or enhanced media publish automatically.

## Acceptance Requirements

- Offline packaged-app test passes with Wi-Fi disabled after first launch.
- WebGPU happy path passes on Apple Silicon macOS.
- Wasm fallback path passes or disables unsupported workloads with clear messaging.
- Large-image cancellation leaves no orphaned derivative file.
- Backup/restore preserves originals, derivatives, accepted crops, generated metadata, and search index behavior.
- Support exports redact embeddings and local AI metadata unless explicitly included.
- Product risk review in `docs/BEST_PRACTICES.md` is rerun with local AI media workflows included.

## Implementation Order

1. Add the local AI media Labs flag and runtime capability probe.
2. Bundle LiteRT.js Wasm locally and prove packaged-app WebGPU/Wasm detection.
3. Select, license-review, bundle, and checksum-validate the image upscaling model.
4. Compile the bundled model from the packaged Labs probe.
5. Implement derivative media metadata and original-preserving model-backed upscaling.
6. Add deterministic media quality preflight.
7. Add smart crop suggestions and derivative crop saving.
8. Add local semantic media search with a licensed embedding model.
9. Add alt-text drafting with operator review.
10. Add offline, packaged Tauri, backup/restore, support-export, and artifact checks.
