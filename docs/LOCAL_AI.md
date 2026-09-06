# Local AI Media

Audience: product, engineering, QA, support, and release owners.

Local AI Media Labs is an opt-in desktop feature. All inference must run inside the packaged app with bundled runtime assets and reviewed model weights. The MVP has no cloud inference, runtime model download, CDN dependency, or hidden remote fallback.

## Current implementation

Implemented:

- Settings-controlled Local AI Media Labs flag.
- Bundled native LiteRT 2.1.6 CPU runtime for Apple Silicon macOS 14+, with the existing `@litertjs/core` Wasm runtime automatically selected on older systems.
- WebGPU and Wasm capability reporting.
- Bundled, checksum-validated Real-ESRGAN-x4plus `w8a8` TFLite model with vendor metadata and a BSD-3-Clause notice.
- Runtime probe that loads LiteRT, reads the model manifest, and compiles the upscaling model.
- Tiled model-backed 4x image upscaling with background processing, progress, and cancellation that terminates the native helper or browser worker before derivative saving begins. Overlapping input context reduces tile joins; source alpha is preserved separately from RGB model inference.
- One shared session interface for the probe and model upscaler, with one model manifest, tiling/pixel pipeline, save command, and session-only processing diagnostics.
- Deterministic fallback upscaling.
- Deterministic media-quality preflight.
- Deterministic crop suggestions saved as derivatives after operator selection.
- Metadata/profile-based local media search using filename, MIME type, media type, dimensions, orientation, broad color tone, brightness, and file-size signals.
- Editable, discardable profile-based alt-text drafts that avoid object, person, identity, demographic, medical, and other sensitive claims. Edited drafts require confirmation before regeneration.
- Shared operation feedback with explicit empty-search results, returned-result counts, persistent status/error regions, and protection against overlapping operations.
- Derivative metadata for source media, operation, model/runtime, dimensions, timestamps, and source/output SHA-256 hashes.
- Release checks for bundled runtime files, model files, metadata, notices, file sizes, and checksums.

## Current limits

- Search is local and profile-based; it does not yet use shared image/text embeddings.
- Alt-text drafts are deterministic profile summaries; they are not model-generated image captions.
- Alt-text edits are temporary session state. Copy reviewed text before quitting; it is not saved to media metadata or attached to posts automatically.
- Cancellation is offered for the runtime probe and model upscaling before its save boundary. It kills/reaps the native helper or terminates the browser worker and discards partial in-memory tiles. Atomic native profile/crop/save operations do not offer a nonfunctional Cancel button. A derivative that has already saved is reported as complete even if refreshing the media list subsequently fails.
- Inputs are limited to 512 × 512px (at most 36 overlapping tiles, 2048 × 2048px output). Larger sources are explicitly rejected, not silently reduced and labeled as 4x upscales. This bounded Labs scope is visible before use and enforced again when saving. Originals remain unchanged. Model-generated fine detail is an estimate, not recovered ground truth.
- Native CPU processing uses at most four threads, reserving CPU capacity for the app. It does not need WebView cross-origin isolation. The older-system Wasm fallback remains capability-gated: threaded Wasm requires shared memory, isolation, and relaxed SIMD. No local HTTP server, hosting change, broad IPC permission, or weaker CSP was introduced. The earlier WebKit investigation is historical context, not a shipping dependency: see [the isolation diagnosis](UX_REVIEW.md#isolation-diagnosis--2026-09-06).
- Model-backed upscaling still requires packaged offline acceptance and human output-quality review for the release candidate.
- WebGPU processing requires supported JSPI GPU-to-CPU pixel readback in the bundled LiteRT 2.5.2 runtime, not merely `navigator.gpu`. The app detects that capability, loads the corresponding bundled runtime variant, and selects Wasm CPU processing when it is absent. The probe reports the actual fallback. No cloud or replacement model is used.
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
- Local model derivatives have a separate 20MB PNG ceiling, validated before base64 decoding and bounded to the expected maximum 2048px dimensions before pixel decoding. Import/upload limits are unchanged. Failed saves clean up their newly created image and thumbnail; successful transaction commits preserve both.

## Using image upscaling

1. Turn on **Settings → Labs → Local AI media** and save settings.
2. Open **Media**, import a static PNG/JPEG up to 512 × 512px, and choose **Upscale** on that image. A runtime probe is optional, not a setup requirement.
3. Social selects its bundled LiteRT engine automatically. Follow the tile progress or cancel before saving. You can navigate and edit a post while inference runs.
4. The separate `AI model upscale x4` image appears in Media. Compare it against the original before using it in a post, especially faces, lettering, and fine texture. AI-estimated details can be wrong.

No account, API key, Python, model download, browser setting, or local server is required. Transparent PNGs retain source transparency. Images above the input limit are rejected clearly rather than silently resized. If processing fails, the original remains available and you can retry; publishing is always a separate user action. A local derivative can be larger than a platform's upload limit, so check the intended platform before publishing.

Use the original when lettering, logos, or exact facial detail must remain faithful. The reviewed model can smooth skin and invent or distort small letters, especially from very small inputs. Increasing pixel dimensions does not recover missing information. Compare the saved derivative at its intended display size and discard it if it changes important detail.

## Packaged-app acceptance

Native LiteRT implementation acceptance on 2026-09-06 passed actual packaged inference, live cancellation/retry, navigation and typing during inference, transparency retention, explicit oversize rejection, source/output hash verification, and a full 512px → 2048px photograph save. The final notarized ux.11 repeated that photograph in 22.7s processing with identical output pixels; the earlier small-fixture and large-image comparisons remain in the local evidence archive. These are local measurements, not broad hardware performance claims. Full source gates passed; the exact packaged helper also passed network-denied inference with reference-identical pixels. Raw captures, fixture databases, and test artifacts are retained locally under `artifacts/litert-upscale-2026-09-06/`, not published with the source.

The clock-related distribution blocker was resolved after the user authorized and authenticated the macOS time correction. A separate rebuild of ux.11 passed Developer ID signing, Apple notarization, stapling, Gatekeeper, deep/strict nested signature checks, and packaged launch smoke. The exact notarized helper also passed network-denied inference and cancellation checks; the artifact identity, submission ID and measurements are in the linked evidence. No security or host-network setting was weakened and no public or installed app was replaced. Labs was saved off again after testing. Clean/older-Mac, full-app offline, and operator quality acceptance remain open.

The subsequent packaged recovery check restored all 30 media files byte-for-byte, preserved derivative metadata, restored a changed preference, and created its automatic safety backup. The exported support log contained none of the tested image names, source/output hashes, pixels, or AI model metadata. Twenty-seven targeted runtime/fallback/cancellation tests also passed. The preliminary visual review and its comparison images are in the same implementation evidence; these checks do not substitute for a clean/older-Mac run or human output-quality approval.

The nondisruptive full-app offline attempt is **not accepted**: launching ux.11 with `sandbox-exec` network denial blocked its native download command, but an uncached external thumbnail still loaded through WKWebView. Cold bundled upscaling succeeded in that session, which validates native-network-denied processing only. Do not treat this profile, the app's online status, or a lack of observed connections as proof that every process is offline.

Local native-debug checks on 2026-09-05 verified preflight, empty/matching searches, alt-text editing/retention/discard/replacement protection, missing-file error recovery, and a separate square crop with verified source/output hashes. These deterministic helper checks do not close the model-backed or packaged/offline gate. See [UX_REVIEW.md](UX_REVIEW.md#local-media-helper-follow-through).

The unchanged signed ux.7 candidate subsequently passed live CPU cancellation without saving output, retry without restart, navigation and composer input during inference, and a verified 32px → 128px derivative. The successful retry measured 155.4s processing for one tile, with a 2105ms maximum UI timer gap: functional interaction passed, but performance is not polished. Initial native-control failures remain recorded, and this small icon does not establish representative quality or offline operation. Labs was saved off in the isolated test workspace. Measurements, output hashes, limitations, and next checks are maintained in [the background processing follow-through](UX_REVIEW.md#background-model-processing-follow-through).

The packaged CSP must include `script-src 'self' 'wasm-unsafe-eval'` for bundled LiteRT WebAssembly. This permits WebAssembly compilation without granting JavaScript `unsafe-eval`, remote scripts, or remote model connections, as described in [Tauri's CSP guidance](https://v2.tauri.app/security/csp/). The first isolated signed UX candidate exposed this missing permission; source asset checks alone had not caught it.

Canvas inputs request anonymous CORS before loading from Tauri's app-owned media asset protocol. This uses the existing app-origin permission and keeps the asset scope at `$APPDATA/media/**`; it does not enable arbitrary file access. Quantized byte output and normalized floating-point output use separate pixel scales.

Use the signed/stapled app, not the Vite development server. Import representative fixtures with the native file picker so macOS can grant access to the selected files:

1. Enable Local AI Media Labs in Settings.
2. Use an operator-approved isolation method that covers both the native app and its WebKit networking helper. When other apps must remain online, use an app-specific outbound firewall rule; do not disable host networking or block the shared WebKit executable globally. **LuLu 4.5.1 did not meet this requirement on the test Mac:** both its main-process and imported process-and-children rules blocked native downloads but still allowed fresh WebKit thumbnails. Those temporary rules were removed; the source-grounded diagnosis and cleanup are in the linked evidence. Do not reuse that configuration as proof of offline operation. An isolated test machine/VM is another option. Disconnecting all active external interfaces requires separate operator approval; turning off Wi-Fi alone is insufficient if another interface remains active. In every case, both native and WebView uncached network controls must fail before calling it offline.
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

Native LiteRT CPU is the preferred execution path for the bundled upscaler on Apple Silicon macOS 14+. LiteRT.js remains the automatic compatibility path for older macOS. Model/integrity/startup errors on a supported native system are reported explicitly; they are not hidden by switching engines or replacing inference with ordinary resizing. A future feature may use ONNX Runtime Web, Transformers.js, or MediaPipe only when it provides a materially better local model path and still meets the same offline, licensing, packaging, privacy, and review requirements.

The Rust boundary accepts only a session ID and fixed-size RGB tile, resolves helper/model paths inside the app bundle, verifies the model SHA-256 against the canonical manifest, and clears the helper environment. One session can run at a time. Each compile/tile operation has a 60-second watchdog; cancellation kills and reaps the helper without waiting for inference. Window destruction closes it, and parent-death detection prevents orphaned work. Only the existing derivative-save command writes media. Native metadata is recorded as `litert_native_2.1.6` / `native_cpu`; historical Wasm derivatives retain their original runtime labels.

Build-time setup is `npm run local-ai:native:prepare`, already part of normal desktop dev/build hooks. It downloads checksum-pinned official wheel/SDK archives once, caches verified archives, extracts only the CPU library, compiles the helper, and emits a package receipt. End users install nothing extra. `npm run local-ai:native:check` rejects stale/missing assets. Dependency notices ship with the bundle; native library updates require reviewing its actual deployment target and dependency notices, not relying on a wheel filename.

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
npm run local-ai:native:prepare
npm run local-ai:native:check
npm run local-ai:assets:check
npm run local-ai:models:check
npm run desktop:ui:test
npm run desktop:release:check
node scripts/benchmark-native-local-ai.mjs --deny-network
```

### Scripted native offline check (macOS)

Run the existing inference benchmark against an exact packaged candidate:

```sh
npm run local-ai:native:offline -- --app "/absolute/path/Dust Wave Social UX Test.app" --output native-offline.json
```

This developer check needs Node and the macOS command-line tools, but no LuLu, administrator privileges, extra network extension, or host-network changes. It does not launch the Social interface, change app data, re-sign the app, or replace the installed version. Without `--app`, it checks the locally prepared helper/model instead of a package.

The script starts fresh TCP and UDP loopback controls over both IPv4 and IPv6. The ordinary parent must reach all four controls before and after the run. A child starts under macOS `sandbox-exec` with `(deny network*)`; all four controls must return explicit permission-denied errors in that child before and after inference. Timeouts, DNS failures, connection refusal, missing controls, and successful traffic are failures, not evidence of isolation. This process sandbox is a macOS developer-test facility, not a portable release security boundary; an unavailable or nonworking sandbox fails the check.

The same sandboxed process family runs the actual bundled helper/model at one and four threads, verifies reference output pixels, interrupts and reaps a running inference, and rejects a malformed frame. It verifies the package signature when `--app` is supplied and reads the model hash from the canonical bundle manifest. Helper/model hashes are checked again after execution. The JSON report records the controls and hashes and exits nonzero on failure. Parent loopback controls prove that this boundary is not host-wide; they are not a claim that an external website is reachable.

**Scope: native LiteRT helper only.** The report deliberately records `full_app_offline_accepted: false`. It does not cover WKWebView, the Rust-to-helper integration, image import/save, multi-tile processing, UI responsiveness, or older-Mac Wasm fallback. Those retain the packaged acceptance steps above. Wrapping the whole app in the same profile does not isolate its launchd-owned WebKit networking helper; removing network entitlements from a separately re-signed sandbox copy also failed to produce a usable interface on the test Mac. Neither method is a full-app offline pass. LuLu 4.5.1's root process-ancestry path skips the responsible-process lookup when its parent PID starts at -1; [the vendor source](https://github.com/objective-see/LuLu/blob/v4.5.1/LuLu/Shared/utilities.m#L125-L240) and read-only inspection of the installed binary agreed. The local evidence archive retains the detailed diagnosis and machine-readable test report.

Support procedures are in [SUPPORT_RUNBOOK.md](SUPPORT_RUNBOOK.md). Product-risk requirements are in [BEST_PRACTICES.md](BEST_PRACTICES.md).

For a repeatable CPU-only diagnostic outside WebKit, use `node scripts/benchmark-local-ai.mjs`, optionally with `--compat` or `--threads=4`. It uses a synthetic tile, the existing runtime/model owner, a four-minute child-process timeout, and no app data or network. Compare output hashes as well as timings. Node feature availability and scheduling differ from the packaged app; always repeat acceptance in the actual signed candidate.
