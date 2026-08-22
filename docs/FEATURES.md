# Dust Wave Social Features

Audience: product, engineering, QA, support, and release operators.

This document describes the current product surface. The canonical operator journeys and their regression IDs are in [USER_FLOWS.md](USER_FLOWS.md); release readiness and manual acceptance belong in [MVP_LAUNCH_PLAN.md](MVP_LAUNCH_PLAN.md); implementation boundaries belong in [ARCHITECTURE.md](ARCHITECTURE.md).

## Desktop workflows

The production desktop interface contains these first-class workflows:

- Dashboard: local status summaries, upcoming and failed posts, provider metrics, audience history, and selectable reporting periods.
- Posts: separate Compose and Post library modes; status tabs, keyword/account/label filters, pagination, bulk selection, post details with Edit in composer, provider previews, error history, contextual schedule/retry-time editing, retry, duplicate, and delete actions.
- Composer: selected accounts, original and account-specific versions, TipTap editing, emoji picker, labels, media drawer, autosave/recovery, provider validation, previews, scheduling, and post-now confirmation.
- Calendar: month/day cells, an hourly week grid, scheduled-post details, composer shortcuts with prefilled dates and times, and detail-to-composer editing with a return path to the originating calendar view.
- Media: local import, drag/drop, URL download, uploaded/stock/GIF tabs, thumbnails, filtering, multi-delete, provider media references, and create-post actions.
- Connections: a Connected accounts tab for progressive provider-specific onboarding, authorization state, refresh, import, disconnect, setup warnings, and redaction-safe exports; plus a Provider setup tab for unified Keychain-backed credentials, configuration, active state, readiness feedback, API versions/tiers, setup packets, and on-demand diagnostics.
- Analytics: provider-specific metrics, audience charts, period summaries, loading/empty/error states, and imported post performance.
- Labels: create, edit, delete, color, filter, and composer assignment.
- Settings: local operator identity, timezone, date/time preferences, week start, default accounts, notifications, and Local AI Media Labs.
- System: health and maintenance first, with lower-frequency recovery, media-tool, log, backup/restore, app-data, notification, and signed-updater controls disclosed on demand. A compact top-right updater action is available from every workflow, changes from check to install when a signed release is found, reports download/verification/installation progress, and relaunches into the installed version.

Migration, schema, release-readiness, raw database, provider-capability, and raw queue panels are intentionally excluded from the production UI.

## Provider support

| Provider | Account and data support | Publishing support | Current limit |
| --- | --- | --- | --- |
| X/Twitter | OAuth 2.0 PKCE, refresh, audience/post imports, and metrics | Text, image, video, GIF, scheduling, and retry where the configured API tier permits | Identical simultaneous posting to multiple X accounts is blocked; live tier acceptance remains required. |
| Facebook Page | Shared Meta OAuth, Page selection, refresh, audience imports, and Page insights | Text, photo, video, scheduling, and retry | Requires approved Meta permissions and real-Page acceptance. |
| Instagram | Professional-account discovery, selection, refresh, media/insight imports, and reports | One static image per post through temporary HTTPS media staging | Business or Creator accounts only; video, reels, stories, carousels, and GIFs are not MVP publishing formats. |
| Mastodon | Per-server app registration, OAuth, refresh, audience/status imports, and metrics | Text and supported image/video/GIF media, scheduling, and retry | Server-specific rules and asynchronous media processing require live acceptance. |
| TikTok | Broker-mediated OAuth credentials, audience/video analytics, reports, and credential revocation | Assisted publishing workflow | Direct API publishing stays disabled until `video.upload` or `video.publish` is approved and deliberately enabled. |

Facebook Groups remain a future Dust Wave extension. Traces in the retained Mixpost package do not constitute a working desktop provider.

## Post and job behavior

- Posts can be drafts, scheduled, processing, published, failed, or retried.
- Provider validation checks text length, required media, media counts, mixed-media restrictions, missing media, account selection, and provider-specific restrictions before jobs are queued.
- Scheduled posts and account imports use durable SQLite jobs and stable idempotency keys.
- The in-app worker reserves due jobs while Dust Wave Social is open.
- Provider rate limits defer only the affected account or application scope.
- Failed posts retain account-level error details and can be retried with a new schedule time.
- Stale processing jobs and failed imports have product-level recovery actions.

Quitting the app stops the local worker. Emergency procedures are documented in [SUPPORT_RUNBOOK.md](SUPPORT_RUNBOOK.md).

## Media

The media library supports:

- App-owned local files and safe copies from operator-selected paths.
- Permitted HTTP(S) downloads with source attribution metadata.
- Unsplash search and download when production credentials and attribution requirements are satisfied.
- Manual/local GIF import.
- Klipy search, preview, selection, and transient publish-time upload.
- Deterministic JPEG/PNG thumbnails and video thumbnails when FFmpeg/FFprobe is available.
- File type/size validation, safe deletion, and orphan cleanup.

Klipy files cannot be saved into the reusable local library without written permission. See [GIF_PROVIDER_DECISION.md](GIF_PROVIDER_DECISION.md).

## Local AI Media Labs

The opt-in Labs surface currently includes:

- Bundled LiteRT.js Wasm runtime and a checksum-validated Real-ESRGAN-x4plus TFLite model.
- Model-backed tiled 4x image upscaling with progress and cancellation.
- Deterministic fallback upscaling and crop derivatives.
- Provider-aware media quality preflight.
- Metadata/profile-based local media search.
- Editable, review-required profile-based alt-text drafts.
- Original-preserving derivatives with model/runtime, dimension, source, and SHA-256 metadata.

True shared image/text embeddings and model-backed image captioning are deferred. Full status and safety constraints are in [LOCAL_AI.md](LOCAL_AI.md).

## Local data and operations

- SQLite stores accounts, posts, versions, media metadata, tags, settings, imports, metrics, audience history, jobs, rate limits, logs, and provider publication records.
- The macOS Keychain stores service and account secrets. SQLite stores references and non-secret configuration.
- Backups contain the database, app-owned media, and a manifest; they exclude Keychain material.
- Restore validates the manifest and creates a safety backup before replacing local data.
- System logs and support exports redact tokens, client secrets, API keys, and refresh tokens.
- Desktop notifications are limited to useful operational events.
- GitHub Releases provide signed updater discovery and installation.

## Deferred work

The following are outside the current MVP or require a new product decision:

- Facebook Group publishing.
- TikTok direct API upload/publishing.
- Instagram video, reels, stories, carousels, and GIF publishing.
- True semantic embedding search and model-backed alt-text captioning.
- Intel or universal macOS distribution.
- Cloud sync, team collaboration, hosted telemetry, or remote AI inference.
- App-lock/password behavior beyond the local operator profile.
- Desktop deep-link/history routing beyond the current workflow state model.
