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
- Native CPU processing uses at most four threads, reserving CPU capacity for the app. It does not need WebView cross-origin isolation. The older-system Wasm fallback remains capability-gated: threaded Wasm requires shared memory, isolation, and relaxed SIMD. No local HTTP server, hosting change, broad IPC permission, or weaker CSP was introduced. The custom-scheme WebKit isolation limitation is not a native-runtime dependency; do not introduce a loopback server to bypass it.
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

[Project status](PROJECT_STATUS.md#evidence-boundaries) owns the latest evidence
and remaining gates; [release operations](RELEASE_OPERATIONS.md) owns signed-build and
updater proof. Do not treat earlier candidate results as acceptance on a new Mac.

Use the exact signed/stapled app, not Vite. Import representative fixtures with
the native file picker so macOS grants access to the selected files.

1. Record app version, Mac chip, macOS version, and original input hashes. Enable Labs.
2. For full-app offline acceptance, use an approved isolated test machine/VM or an app-specific method proven to cover both the native app and WebKit networking. Both uncached controls must fail. Never disable host networking while other apps need it, or globally block the shared WebKit executable. Changing all active network interfaces requires separate approval.
3. Run the optional runtime probe and record the actual native/Wasm path or error.
4. Upscale a transparent PNG and representative photographs within the 512px limit; check the 4× output dimensions and derivative metadata.
5. Test progress, cancellation, retry, navigation/typing during inference, and no partial derivative after cancellation.
6. Verify unchanged originals and source/output hashes. Review faces, text, and fine detail at intended display size; discard misleading output.
7. Exercise preflight, crop suggestions, property search, and editable/discardable alt-text drafts.
8. Back up and restore originals, derivatives, and metadata; verify the safety copy and support-export redaction.
9. Turn Labs off again when finished and record pass/fail/not tested for each boundary.

**Known failed isolation approaches (2026-09-06):** a whole-app
`sandbox-exec` profile and LuLu 4.5.1 main-process/process-tree rules blocked native
downloads but allowed fresh WKWebView thumbnails. Those temporary rules were
removed. Neither approach is full-app offline evidence. Wi-Fi off alone also
does not establish isolation when another interface is active.

The scripted native-helper check below is intentionally narrower. Human quality,
older-system execution, accessibility, and representative data recovery require
their own outcomes.

## Runtime and model policy

Native LiteRT CPU is the preferred execution path for the bundled upscaler on Apple Silicon macOS 14+. LiteRT.js remains the automatic compatibility path for older macOS. Model/integrity/startup errors on a supported native system are reported explicitly; they are not hidden by switching engines or replacing inference with ordinary resizing. A future feature may use ONNX Runtime Web, Transformers.js, or MediaPipe only when it provides a materially better local model path and still meets the same offline, licensing, packaging, privacy, and review requirements.

The Rust boundary accepts only a session ID and fixed-size RGB tile, resolves helper/model paths inside the app bundle, verifies the model SHA-256 against the canonical manifest, and clears the helper environment. One session can run at a time. Each compile/tile operation has a 60-second watchdog; cancellation kills and reaps the helper without waiting for inference. Window destruction closes it, and parent-death detection prevents orphaned work. Only the existing derivative-save command writes media. Native metadata is recorded as `litert_native_2.1.6` / `native_cpu`; historical Wasm derivatives retain their original runtime labels.

Build-time setup is `npm run local-ai:native:prepare`, already part of normal desktop dev/build hooks. It downloads checksum-pinned official wheel/SDK archives once, caches verified archives, extracts only the CPU library, compiles the helper, and emits a package receipt. End users install nothing extra. `npm run local-ai:native:check` rejects stale/missing assets. Dependency notices ship with the bundle; native library updates require reviewing its actual deployment target and dependency notices, not relying on a wheel filename.

### WebKit compatibility constraints

The bundled Wasm path needs only `script-src 'self' 'wasm-unsafe-eval'` for
WebAssembly compilation; JavaScript `unsafe-eval`, remote scripts/models, and
broad filesystem permissions remain disallowed. Image inputs request anonymous
CORS before loading the app-owned media asset protocol, whose scope stays at
`$APPDATA/media/**`. Quantized-byte and normalized-float outputs use explicit
typed-array conversion rather than value heuristics.

LiteRT's GPU readback requires JSPI, not merely `navigator.gpu`. The worker loads
the reviewed bundled glue/Wasm paths through the existing runtime owner. The
custom `tauri://localhost` scheme did not expose shared memory/isolation in the
tested WKWebView, even with COOP/COEP; native CPU avoids that dependency. Retain
these compatibility regression tests when upgrading the runtime.

Every added model must record:

- Model name and version.
- Source URL and source package version.
- License and redistribution permission.
- Intended local-only use.
- File size and SHA-256 checksum.
- Runtime and supported execution paths.
- Known quality and bias limits.

Update [Third-party notices](THIRD_PARTY_NOTICES.md) and the model manifest before committing new weights.

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
npm run local-ai:native:offline -- --app "/absolute/path/Dust Wave Social.app" --output native-offline.json
```

This developer check needs Node and the macOS command-line tools, but no LuLu, administrator privileges, extra network extension, or host-network changes. It does not launch the Social interface, change app data, re-sign the app, or replace the installed version. Without `--app`, it checks the locally prepared helper/model instead of a package.

The script starts fresh TCP and UDP loopback controls over both IPv4 and IPv6. The ordinary parent must reach all four controls before and after the run. A child starts under macOS `sandbox-exec` with `(deny network*)`; all four controls must return explicit permission-denied errors in that child before and after inference. Timeouts, DNS failures, connection refusal, missing controls, and successful traffic are failures, not evidence of isolation. This process sandbox is a macOS developer-test facility, not a portable release security boundary; an unavailable or nonworking sandbox fails the check.

The same sandboxed process family runs the actual bundled helper/model at one and four threads, verifies reference output pixels, interrupts and reaps a running inference, and rejects a malformed frame. It verifies the package signature when `--app` is supplied and reads the model hash from the canonical bundle manifest. Helper/model hashes are checked again after execution. The JSON report records the controls and hashes and exits nonzero on failure. Parent loopback controls prove that this boundary is not host-wide; they are not a claim that an external website is reachable.

**Scope: native LiteRT helper only.** The report deliberately records `full_app_offline_accepted: false`. It does not cover WKWebView, the Rust-to-helper integration, image import/save, multi-tile processing, UI responsiveness, or older-Mac Wasm fallback. Those retain the packaged acceptance steps above. Wrapping the whole app in the same profile does not isolate its launchd-owned WebKit networking helper; removing network entitlements from a separately re-signed sandbox copy also failed to produce a usable interface on the test Mac. Neither method is a full-app offline pass. LuLu 4.5.1's root process-ancestry path skips the responsible-process lookup when its parent PID starts at -1; [the vendor source](https://github.com/objective-see/LuLu/blob/v4.5.1/LuLu/Shared/utilities.m#L125-L240) and read-only inspection of the installed binary agreed. Superseded diagnostic files were moved to Trash during cleanup; current release-native reports are retained with the release assets.

Support procedures are in [SUPPORT_RUNBOOK.md](SUPPORT_RUNBOOK.md). Product-risk requirements are in [BEST_PRACTICES.md](BEST_PRACTICES.md).

For a repeatable CPU-only diagnostic outside WebKit, use `node scripts/benchmark-local-ai.mjs`, optionally with `--compat` or `--threads=4`. It uses a synthetic tile, the existing runtime/model owner, a four-minute child-process timeout, and no app data or network. Compare output hashes as well as timings. Node feature availability and scheduling differ from the packaged app; always repeat acceptance in the actual signed candidate.
