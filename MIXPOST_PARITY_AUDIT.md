# Mixpost Parity Audit

Status date: 2026-06-26

This audit uses the bundled Mixpost Lite code in this repository plus the upstream Mixpost Lite tag list as the parity source of truth. The latest upstream tag verified for this pass is `2.6.0`.

- `routes/web.php`
- `resources/js/Pages/**`
- `resources/js/Components/**`
- `src/SocialProviderManager.php`
- `config/mixpost.php`

The active provider parity target is X/Twitter, Facebook Page, and Mastodon. `facebook_group` appears in config and reporting components, but it is not registered by `src/SocialProviderManager.php`; treat it as out of scope unless Dust Wave explicitly wants to add it.

## Current Conclusion

The local Dust Wave Social implementation now covers every Mixpost parity item that can be completed without live provider credentials, Apple signing/notarization, target-device runtime checks, or final human visual acceptance. Complete real-world parity still depends on the external work in `DUSTWAVE_FINISH_LINE_CHECKLIST.md`, especially provider portal setup, provider OAuth approvals, live publishing/import acceptance, signing, updater key setup, and visual QA. Dust Wave now automates the provider setup packet and account onboarding packet from the desktop app, but the provider portals and live accounts still have to be operated by a human.

Ethical product acceptance is now part of finish-line parity. The reusable standard lives in `docs/BEST_PRACTICES.md` and must be applied before shipping workflows that affect publishing, automation, account connection, credentials, imports, media, reporting, backup/restore, notifications, support exports, or provider behavior.

The desktop shell now has first-class Mixpost-equivalent views for Dashboard, Posts, Calendar, Media, Accounts, Services, Reports, Tags, Settings, Profile, and System, backed by a route parity test that maps every named route in `routes/web.php` to a desktop workflow or intentional desktop replacement.

Removed from the production-facing desktop surface:

- Release Readiness
- Migration Milestones
- Provider Capability Map
- Local Data Store import/diagnostics/backup panel
- Database status and initial schema panels
- Raw queue/job and rate-limit control commands
- Mixpost SQLite import and verification Tauri commands
- Diagnostics export Tauri commands
- Hidden local-backup utility code

Internal database methods and tests for migration and diagnostics have been removed from the active desktop code. Backup/restore has been promoted to a real user-facing System workflow with manifest-based backups and safety backups before restore.

## Parity Matrix

| Mixpost workflow | Desktop status | Remaining parity work |
| --- | --- | --- |
| Navigation/page shell | Covered locally | Desktop now separates workflows into route-like views matching the Mixpost surface, including Profile. A Rust parity test maps every named route in `routes/web.php` to an equivalent desktop workflow or intentional desktop replacement. Replace this view-state shell with real route/deep-link handling only if needed for desktop history. |
| Dashboard | Covered locally | Desktop has summary cards, upcoming posts, failed posts, provider summaries, Mixpost-style account-avatar analytics selector, 7/30/90-day period tabs, loading/empty/error treatment, provider metric cards, and interactive audience chart summaries. Remaining work is pixel-level visual QA. |
| Reports endpoint | Covered locally | Rust aggregates X/Twitter, Facebook Page, and Mastodon metrics, and the desktop UI now uses provider-specific metric cards, X Free-tier warning, mutually exclusive loading/error/empty states, a lazy Chart.js audience line chart, selected-point controls, period change, high, and average summaries. Remaining work is live-data visual QA against real provider accounts. |
| Accounts | Covered locally, pending live acceptance | Provider connection cards for X/Twitter, Facebook Page, and Mastodon now live in a Mixpost-style Add Account modal and first-card Add Account entry, the manual raw account form is no longer visible, X/Facebook cards show service-ready warnings before OAuth, connected accounts use avatar cards with authorization state, account deletion is confirmed, Facebook Page selection uses entity cards with select-all/clear controls, and the Accounts screen can copy an onboarding intake CSV plus live connected-account inventory. Remaining work is live OAuth acceptance. |
| Services | Covered locally, pending live acceptance | Desktop now has Mixpost-style provider tabs for Facebook, X, Unsplash, and Klipy with exact credential labels, keychain saves, active toggles, validated X tier, validated Facebook API-version persistence, per-provider setup copy, and bulk setup packets for all or missing services. Facebook OAuth/page/publish/import/report paths now use the saved API version, and X Free-tier imports skip unsupported post endpoints. Keep Mastodon app registration on Accounts. Klipy is the MVP GIF provider; Tenor is retired context only because Google stopped accepting new API clients in January 2026 and the third-party API shutdown window has passed. |
| Posts list | Covered locally | The desktop Posts view now uses query-backed status tabs, keyword filtering, Mixpost-style account/tag filter popovers, selectable rows, page select/clear, Mixpost-style selected bar, pagination, media thumbnails, labels, account chips, confirmed deletes, row actions, read-only post detail modal with provider previews/history, visible failed-post error messages, per-row schedule drafts, retry-now, and bulk delete. Remaining work is exact pixel-level table-density QA. |
| Post create/edit | Covered locally | Draft, update, schedule, post-now confirmation, duplicate, account picker cards, explicit original/account version tabs with create/remove behavior, TipTap-based editor, lazy-loaded emoji-mart picker, composer label search/create/select/remove, composer media drawer with import/select/remove, advisory provider validation, X simultaneous-posting prevention, Mixpost-style bottom schedule/action bar, schedule/retry controls, publishing locks for non-editable statuses, shared account-scoped provider preview cards, local composer autosave/recovery, and invalid schedule handling exist. Remaining work is visual QA and any deliberately chosen future content-block additions. |
| Provider previews | Covered locally | Desktop now renders shared account-scoped preview cards with per-account text, provider identity treatment, character counters, Twitter representative engagement metrics, Facebook reaction/toolbar treatment, Mastodon icon rows, provider-specific video play overlays, and provider-shaped media gallery layouts. Remaining work is visual QA against the original components and live media edge cases. |
| Calendar | Covered locally | Desktop now has selectable month/day calendar cells, a Mixpost-style hourly Week time grid, weekday headers, dense bordered layouts, status-colored clickable post chips that open the shared post-detail preview/history modal, previous/today/next toolbar controls, a period title, a detailed agenda list with View actions, selected-date New Post, and week-slot New Post shortcuts that pre-fill the composer schedule time. Remaining work is final visual QA against Mixpost month/week layouts. |
| Media library | Covered locally, pending live acceptance | Desktop now separates Uploaded, Stock, and GIF source tabs, supports selected-media Create Post, local import with Mixpost-compatible picker filters/drop target, multi-file import queue with per-file success/failure results, URL download, Unsplash, Klipy GIF search, source-aware external empty states, delete confirmation, selected upload multi-delete, operation progress text, thumbnails, filtering, and composer-side media import/select/remove. Klipy selections now use provider references plus transient publish-time fetch/upload per `docs/GIF_PROVIDER_DECISION.md`; permanent Klipy reusable-download storage is blocked. Remaining work is visual QA against Mixpost media cards, target-machine media runtime checks, and live Klipy credential acceptance. |
| Tags | Covered locally | Create, update, delete, colors, post linking, composer label search/create/select/remove, and Mixpost-style post list/calendar label filters exist. Remaining work is live-data visual QA. |
| Settings | Covered locally | Desktop now matches Mixpost's grouped Notifications and Time settings layout with admin email, desktop notification preference/test action, timezone selector, time format, and first-day-of-week controls. Dust Wave publishing defaults remain in a separate product panel. |
| Profile/auth | Desktop replacement | Desktop now has a Profile view for local operator name/email. Laravel password, logout, and CSRF flows are web-session mechanics and are intentionally non-applicable unless Dust Wave adds an app-lock feature. |
| System status/logs | Covered locally | Desktop has health, desktop-safe technical details, FFmpeg/FFprobe availability status, maintenance, stale recovery, failed-import retry, local system logs with export/clear, a real backup/restore workflow with manifest validation, desktop notifications, Software Updates status, Copy Info, and Copy App Data Path actions. Keep diagnostics, migrations, and raw queues out of the production UI unless promoted as product features. |
| Background publishing/imports | Covered locally, pending live acceptance | Durable queued publishing, account imports, provider validation, X simultaneous-posting validation, rate-limit deferral, app-open auto-run, stale-job recovery, and product-level retry for failed account imports exist. Remaining work is live-provider acceptance for each connected account and recovery checks for provider-specific failures. |
| Media processing | Covered locally, pending target-Mac acceptance | App-owned media storage, metadata, MIME/size enforcement, Mixpost-compatible picker types, deterministic image thumbnails, video upload without FFmpeg hard-failure, video thumbnail test coverage, System-view FFmpeg/FFprobe status, Tauri sidecar packaging, Apple Silicon LGPL-only FFmpeg/FFprobe sidecars, and third-party notices exist. Remaining work is clean Apple Silicon target-Mac verification. Intel/universal macOS builds are out of MVP scope by Dust Wave decision. |
| Notifications | Desktop replacement | Desktop uses health/logs, a workspace attention strip, a persisted desktop notification preference, a test notification action, and native desktop notifications instead of Laravel mail. Unauthorized accounts, failed posts, failed background work, active provider limits, and scheduled-publish worker outcomes now surface without adding a Laravel-style mail pipeline. |

## Finish-Line Product Tasks

1. Keep the desktop navigation aligned to Mixpost user workflows: Dashboard, Posts, Calendar, Media, Accounts, Services, Reports, Tags, Settings, Profile, and System.
2. Treat additional local Mixpost behavior as follow-on work only when manual acceptance finds a concrete mismatch.
3. Keep the post composer aligned to Mixpost's data model and interaction model: accounts first, explicit original/account-specific version tabs, TipTap editing, emoji-mart picker, composer labels, composer media drawer behavior, provider validation, previews, autosave, invalid schedule handling, and the schedule/post-now action bar now exist.
4. Verify the posts index density and bulk-action details against Mixpost during visual QA.
5. Verify the calendar month/week density against Mixpost during visual QA.
6. Verify media library per-file upload errors and external media empty states during visual QA and live credential tests. Treat GIF search as non-production until Klipy attribution, content filters, production credentials, and live non-persistent publish behavior are accepted.
7. Finish the accounts page with live OAuth acceptance for provider-specific OAuth paths, refresh/delete actions, and unconfigured-service warnings.
8. Validate service active/inactive state against live credentials during provider acceptance testing.
9. Finish dashboard/report visual QA using the provider report components, chart behavior, empty states, and metric labels from Mixpost.
10. Keep the desktop Profile replacement limited to local name/email unless Dust Wave explicitly wants an app-lock/password feature.
11. Keep the route parity test current whenever `routes/web.php` or the desktop workflow surface changes.
12. Run live-provider acceptance for X/Twitter, Facebook Page, and Mastodon: connect, refresh, import, publish text, publish image, publish video where supported, schedule, retry failure, and recover expired/invalid credentials.
13. Keep migration, release, schema, diagnostics, and raw queue tooling out of the production UI unless each item is promoted through a separate product decision.
14. Keep System status aligned to current Mixpost Lite support diagnostics, including media-tool availability, logs, service readiness, and scheduled/background work health.
15. Complete the ethical product risk review in `docs/BEST_PRACTICES.md` before public release and before any feature that materially changes social publishing, automation, provider handling, backups, support exports, notifications, or local data behavior.

## Manual Decisions Needed

- Confirm final provider credentials, app-review permissions, and live test accounts for X/Twitter, Facebook Page, Mastodon, Unsplash, and Klipy. Complete the Klipy production gate in `docs/GIF_PROVIDER_DECISION.md` before production GIF search acceptance.
- Verify the staged Apple Silicon LGPL-only FFmpeg/FFprobe sidecars on a clean target Mac. Intel/universal release sidecars are not required for MVP.
- Confirm the final GitHub repository slug and updater signing key before enabling live updater config.
- Assign an owner for ethical red-flag triage during release acceptance, including misinformation, impersonation, harassment, spam, doxxing, surveillance, account takeover, surprise data collection, and accidental public posting risks.
