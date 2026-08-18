# Local AI Media

Audience: product, engineering, QA, support, and release owners.

Local AI Media Labs is an opt-in desktop feature. All inference must run inside the packaged app with bundled runtime assets and reviewed model weights. The MVP has no cloud inference, runtime model download, CDN dependency, or hidden remote fallback.

## Current implementation

Implemented:

- Settings-controlled Local AI Media Labs flag.
- Bundled `@litertjs/core` Wasm runtime assets.
- WebGPU and Wasm capability reporting.
- Bundled, checksum-validated Real-ESRGAN-x4plus `w8a8` TFLite model with vendor metadata and a BSD-3-Clause notice.
- Runtime probe that loads LiteRT, reads the model manifest, and compiles the upscaling model.
- Tiled model-backed 4x image upscaling with progress and cancellation between tiles.
- Deterministic fallback upscaling.
- Deterministic media-quality preflight.
- Deterministic crop suggestions saved as derivatives after operator selection.
- Metadata/profile-based local media search using filename, MIME type, media type, dimensions, orientation, broad color tone, brightness, and file-size signals.
- Editable profile-based alt-text drafts that avoid object, person, identity, demographic, medical, and other sensitive claims.
- Derivative metadata for source media, operation, model/runtime, dimensions, timestamps, and source/output SHA-256 hashes.
- Release checks for bundled runtime files, model files, metadata, notices, file sizes, and checksums.

## Current limits

- Search is local and profile-based; it does not yet use shared image/text embeddings.
- Alt-text drafts are deterministic profile summaries; they are not model-generated image captions.
- Model-backed upscaling still requires packaged offline acceptance and human output-quality review for the release candidate.
- WebGPU availability depends on the packaged WebView and target Mac. Wasm is the fallback path.
- Local AI tools support app-owned static images, not GIFs, videos, external provider references, or missing files.

True embedding search and model-backed captioning are deferred until a suitable local model passes quality, licensing, file-size, performance, offline-packaging, and product-risk review.

## Safety and data rules

- Preserve every original file. Save enhanced or cropped output as a separate derivative.
- Never publish a generated derivative or alt-text draft automatically.
- Keep alt text visibly generated, editable, removable, and subject to operator review.
- Do not send media, thumbnails, hashes, embeddings, search terms, captions, or derivatives to a remote inference service.
- Do not include generated metadata or future embeddings in support exports unless the operator explicitly includes them.
- Include derivatives and their metadata in backups, but continue excluding Keychain secrets.
- On cancellation or failure, remove partial output and do not create a derivative record.
- Treat poor output as a model-quality acceptance failure; keep the original and delete the derivative if needed.

## Packaged-app acceptance

Use the signed/stapled app, not the Vite development server:

1. Enable Local AI Media Labs in Settings.
2. Disable Wi-Fi.
3. Run the LiteRT capability probe and record WebGPU, Wasm fallback, or the exact error.
4. Upscale representative small, medium, and large images.
5. Test progress, cancellation, retry, and cleanup of partial output.
6. Confirm the original remains unchanged.
7. Confirm every derivative records source media, runtime/model, dimensions, operation, and hashes.
8. Exercise preflight, crop suggestions, local search, and alt-text drafting.
9. Review image quality and alt-text usefulness manually.
10. Back up and restore the resulting originals, derivatives, and metadata.
11. Export system logs and confirm local-AI data is redacted as designed.

The release gate remains open until this workflow passes in the current packaged candidate.

## Runtime and model policy

LiteRT.js is the primary runtime for the bundled upscaler. A future feature may use ONNX Runtime Web, Transformers.js, or MediaPipe only when it provides a materially better local model path and still meets the same offline, licensing, packaging, privacy, and review requirements.

Every added model must record:

- Model name and version.
- Source URL and source package version.
- License and redistribution permission.
- Intended local-only use.
- File size and SHA-256 checksum.
- Runtime and supported execution paths.
- Known quality and bias limits.

Update `../THIRD_PARTY_NOTICES.md` and the model manifest before committing new weights.

## Verification commands

```sh
npm run local-ai:assets:check
npm run local-ai:models:check
npm run desktop:ui:test
npm run desktop:release:check
```

Support procedures are in [SUPPORT_RUNBOOK.md](SUPPORT_RUNBOOK.md). Product-risk requirements are in [BEST_PRACTICES.md](BEST_PRACTICES.md).
