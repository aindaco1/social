# Dust Wave Finish-Line Checklist

This is the external and acceptance work remaining after the local Tauri/Rust migration foundation and Pool/Store design-language pass. The app now automates the provider setup packet, account onboarding packet, local Developer ID signing wrapper, and updater key generation from live local state. Provider portals, OAuth approvals, final GitHub release hosting, Apple notarization issuer details, and live-provider acceptance still require an operator.

## Design And Brand

- MVP decision: ship with the Gambado typography treatment from Pool/Store. Dust Wave accepts the follow-up responsibility for confirming redistribution rights for the Gambado font files before broader public distribution.
- The Gambado OpenType files are bundled in `resources/fonts` and registered in `resources/css/fonts.css`; rebuild and visually compare headings against Pool and Store.
- The desktop app now uses the `dust-wave-square` logo from the Pool assets for the sidebar mark, Tauri PNG icons, and macOS `.icns`.
- Review the desktop UI on a real Mac at 1024px, 1280px, and wide desktop widths, then compare against Pool/Store for typography, control density, panel rhythm, and button treatment.
- Keep production labels focused on social account management, publishing, media, reports, settings, and system logs; do not reintroduce migration milestones, release readiness, schema, or raw queue inspection panels to the shipped app.

## Apple Signing And Distribution

- Enroll or confirm access to the Dust Wave Apple Developer account.
- Local Developer ID signing is automated from the sibling iCloud `Apple Auth` folder when it contains `developer-id-application.p12` and `apple-p12-password.txt`.
- App Store Connect API key discovery is automated from `Apple Auth/AuthKey_<key-id>.p8`.
- Add the App Store Connect issuer UUID to `Apple Auth/apple-api-issuer.txt`; this is the current blocker for automated notarization.
- GitHub release secrets and variables have been configured for `aindaco1/social` except `APPLE_API_ISSUER`, which still needs the issuer UUID.
- Run `npm run desktop:release:check` with signing variables present and confirm the build, format check, and Rust tests pass.
- Run `npm run desktop:release:build:signed:with-media` to build a signed Apple Silicon `.app` and signed `.dmg` with bundled LGPL-only media sidecars.
- Run `npm run desktop:macos:notarize` after the issuer UUID is available, then confirm the stapled DMG passes `spctl`.
- Local unsigned builds are automatically ad-hoc signed for smoke testing, but ad-hoc signing is not a distribution substitute.
- The signed wrapper creates the DMG directly with `hdiutil` instead of Tauri's local DMG helper, which previously hung.
- Install the notarized DMG on a clean Mac and confirm Gatekeeper opens it without warnings.

## Updater

- Use GitHub Actions and GitHub Releases for update artifacts.
- Tauri updater signing keys are generated outside the repo with `npm run desktop:updater:keys` and stored in `Apple Auth`.
- The final GitHub release repository slug is `aindaco1/social`, and local `origin` points there.
- `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `TAURI_UPDATER_PUBLIC_KEY`, and `DUSTWAVE_RELEASE_REPO` are configured in GitHub secrets/variables.
- Use `npm run desktop:updater:check` or `npm run desktop:release:build:signed:with-media-and-updater`; local scripts infer `aindaco1/social` from `origin` when the environment does not set `DUSTWAVE_RELEASE_REPO`.
- The generated updater overlay lives at `src-tauri/tauri.updater.generated.conf.json` and is ignored by git. Keep `src-tauri/tauri.updater.example.json` as the documented shape.
- Updater builds generate `src-tauri/target/release/bundle/latest.json` beside the signed macOS artifacts. The manual GitHub Actions workflow can now fail closed when updater artifacts are requested but missing.
- Publish a test update from version `0.1.0` to a higher version and verify the installed app detects, downloads, validates, installs, and relaunches.
- Define rollback policy for a bad release, including whether old installers remain available.

## Provider Credentials

- In the Services screen, use **Copy All Setup** to copy the full provider setup packet, or **Copy Missing** to copy only providers that are not active and configured. The packet includes setup URLs, docs URLs, callback URLs, scopes, config values, and missing/saved credential status without exposing secret values.
- Create or verify the X/Twitter developer app, callback URLs, OAuth scopes, client ID, and client secret using the copied Services packet.
- Create or verify the Meta/Facebook app, callback URLs, page permissions, page publishing permissions, and app review requirements using the copied Services packet.
- Register the Mastodon app flow for each server Dust Wave will use, or confirm dynamic registration is acceptable for production.
- Create Unsplash API credentials for stock-image search.
- Create Klipy API credentials for GIF search. Dust Wave selected Klipy for MVP implementation because it offers a Tenor migration path, test keys, production access, attribution guidance, content filters, and optional ads from the Partner Panel.
- Do not plan MVP around Tenor. Google stopped accepting new Tenor API clients in January 2026 and the third-party API shutdown window has passed. Treat Tenor as retired Mixpost context only.
- Complete the Klipy production gate in `docs/GIF_PROVIDER_DECISION.md` before treating GIF search as release-ready. Klipy's API terms allow search, preview, selection, and posting, but prohibit permanent Klipy content storage beyond search-result thumbnails in the user's app copy. The app uses non-reusable provider references plus transient publish-time fetch/upload for Klipy content unless Klipy gives written permission for permanent media-library imports.
- Store each provider credential through the app credential forms or the intended secret bootstrap flow, not directly in SQLite.
- Verify system logs and provider error output redact all access tokens, refresh tokens, client secrets, and API keys.

## Dust Wave Account Onboarding

- In the Accounts screen, use **Copy Intake CSV** to create the blank account inventory sheet with provider, handle/page ID, owner, posting, import, and notes columns.
- Use **Copy Plan** after any accounts are connected to copy the current service readiness, connected-account counts, intake CSV, and connected-account inventory CSV.
- List every Dust Wave social account to manage in the copied intake CSV, including provider, handle/page ID, owner, required permissions, and whether posting is allowed.
- Connect each account through the desktop app and verify the app stores only keychain/secret references in local data.
- Refresh each connected account and confirm username, provider ID, avatar metadata, authorization status, and account metrics.
- Queue account imports for all connected supported accounts and verify imported posts, metrics, and audience snapshots.
- Confirm unsupported providers are documented as manual workflows or future provider work.

## Live Publishing Validation

- Publish a text-only test post to each provider.
- Publish a photo post to each provider that supports photos.
- Publish a video post to each provider that supports videos.
- Publish a GIF/media test where supported.
- Schedule a future post, let the local worker publish it, and verify the final provider URL or remote ID is captured.
- Force or simulate rate limits and confirm jobs defer, retry, and recover as expected.
- Revoke one provider token and confirm health checks, failed jobs, and reconnect flows behave correctly.
- Verify per-provider validation for character limits, media counts, media type restrictions, and missing account credentials.
- Verify X/Twitter refuses simultaneous posting to multiple X accounts and guides the operator to create separate posts.

## Ethical Product Acceptance

- Run the product risk review in `docs/BEST_PRACTICES.md` before the first public release and before any later release that changes publishing, automation, account connection, media, backup, reporting, notifications, or provider behavior.
- Confirm the release does not make misinformation, impersonation, spam, harassment, doxxing, or coordinated abuse materially easier without a documented mitigation.
- Red-team the publish-now, schedule, duplicate, retry, bulk delete, account connection, backup/restore, media import, and support-export workflows from the perspective of an unauthorized or malicious operator.
- Verify all externally visible actions have clear operator intent: account selection, provider preview, schedule time, publish-now confirmation, retry behavior, and final provider status.
- Confirm setup packets, onboarding packets, logs, support exports, backups, and screenshots do not expose access tokens, refresh tokens, client secrets, API keys, or sensitive account metadata beyond what the operator intentionally exported.
- Confirm desktop notifications are limited to meaningful operational events such as failed publishing, expired credentials, scheduled-post outcomes, and system health issues.
- Document the manual emergency procedure for stopping scheduled publishing, disconnecting a compromised account, revoking provider credentials, preserving logs, and restoring from backup.
- Assign an owner for ethical red-flag triage during release acceptance. Any red flag from `docs/BEST_PRACTICES.md` must have an owner, mitigation, and ship/no-ship decision before release.

## Mixpost Data Migration

- Production migration utilities have been removed from the shipped app. If Dust Wave later needs to import an existing Mixpost database, plan a separate one-time utility with its own dry run, backup, verification, and rollback steps.
- Keep any legacy Laravel/Mixpost system available until live publishing and historical reporting have passed acceptance.

## Media And Local Runtime

- MVP decision: Apple Silicon macOS only. Intel and universal macOS sidecars are out of scope unless Dust Wave later changes distribution targets.
- Apple Silicon LGPL-only FFmpeg/FFprobe sidecars are staged from official FFmpeg 8.1.2 source with `npm run desktop:media:build-lgpl`.
- Record the FFmpeg/FFprobe versions, configure lines, binary source, source archive/commit, and license output in `THIRD_PARTY_NOTICES.md` for each release. The current Apple Silicon sidecar record is filled in.
- Re-stage approved portable media binaries when updating FFmpeg with `DUSTWAVE_FFMPEG_BINARY=/path/to/ffmpeg DUSTWAVE_FFPROBE_BINARY=/path/to/ffprobe npm run desktop:media:prepare`.
- Build release artifacts with bundled media tools using `npm run desktop:release:build:with-media`.
- Confirm System reports bundled FFmpeg and FFprobe on a clean target Mac with no Homebrew FFmpeg installation.
- Import local images, videos, and GIFs into the app and confirm thumbnails, sizes, MIME types, and cleanup behavior.
- Download external media from URL and Unsplash using production credentials. Validate Klipy GIF search separately: search and preview should work with production credentials, selected Klipy items must not be saved as reusable media-library files, and any publish-time file fetch must be temporary, deleted after the publish attempt, and excluded from backups/support exports.
- Run orphaned media cleanup and confirm referenced media files are not removed.
- Backup/restore is now a real production System workflow with manifest validation. Confirm backups include the database, media, and manifest, then restore one on a clean machine or clean app-data directory.
- Confirm OS keychain secrets are intentionally excluded from backup files and that restored accounts prompt for reconnect when needed.

## QA And Acceptance

- Run `npm ci` from a clean checkout.
- Run `npm run desktop:release:check` and confirm Vue build, Rust formatting, and Rust tests pass.
- Run `npm run desktop:release:build` and confirm the macOS `.app` and `.dmg` are produced.
- Run `npm run desktop:smoke:launch` against the packaged `.app` before deleting local build artifacts.
- Complete the ethical product acceptance section before signing release artifacts.
- Test first launch with no existing app-data directory.
- Test relaunch with existing app-data and pending jobs.
- Test settings save/load, default accounts, timezone, date format, week start, and desktop notification preference.
- Test system logs refresh, export, and clear.
- Test Copy App Data Path from System and confirm it gives support the correct local folder without exposing raw database UI.
- Test bulk delete, duplicate post, validate post, retry failed post, recover stale jobs, and queue account imports.
- Use `docs/SUPPORT_RUNBOOK.md` for the support workflow around app-data discovery, failed publishing, backup/restore, emergency scheduled-publishing stops, and support export hygiene.
- Do a final visual QA pass against Pool and Store with the bundled Gambado files.

## CI And Repository Hygiene

- Review untracked files before staging. Do not commit `src-tauri/target`, `resources/desktop/dist`, notarized DMGs, or local app-data.
- Add CI secrets for signing, notarization, updater signing, and provider smoke-test credentials only in secure secret stores.
- Run the desktop CI workflow manually once with secrets present.
- Confirm generated release artifacts are uploaded only from CI or a controlled local release machine, preferably through the draft GitHub Release path in `.github/workflows/desktop.yml`. Use the workflow inputs for media sidecars and updater artifacts only after the corresponding secrets and sidecars are ready.
- Tag the first signed release with a clear version, release notes, and migration warnings.

## Operations

- Define who owns provider credentials, Apple credentials, update hosting, and release approvals.
- Define backup frequency and retention for local app data.
- Define support policy for corrupted local databases, failed migrations, revoked tokens, and missed scheduled posts.
- Decide when Laravel/Mixpost can be archived after parity, migration verification, and live-provider acceptance are complete.
- Keep a final go/no-go checklist for launch day: signed build installed, accounts connected, test publish passed, backup exported, updater tested, and rollback installer available.
