# Dust Wave Social MVP Launch Plan

Updated: 2026-08-18

Audience: Dust Wave operators and release maintainers preparing the Apple Silicon macOS release.

This is the single source of truth for readiness, manual acceptance, release artifacts, publishing, and rollback. Product capabilities are documented in [FEATURES.md](FEATURES.md); implementation boundaries are documented in [ARCHITECTURE.md](ARCHITECTURE.md).

## Launch definition

The MVP is ready when Dust Wave can:

- Install and relaunch a signed, notarized, stapled Apple Silicon app on a clean Mac.
- Connect production X/Twitter, Facebook Page, Instagram, Mastodon, and TikTok accounts.
- Publish and schedule representative supported content and recover from provider failures.
- Import supported audience, post, Instagram insight, and TikTok broker analytics.
- Use local media and opt-in Local AI Media Labs in the packaged app without network inference.
- Back up and restore app-owned data without exposing Keychain secrets.
- Install a higher-version signed update and roll back to a known-good build.
- Pass product-risk, security, support, and visual acceptance.

Run the readiness audit whenever launch state changes:

```sh
npm run mvp:launch:readiness
```

Implementation/infrastructure readiness is not the same as live-provider or launch acceptance. Manual items remain open until an operator records evidence from the real provider, account, artifact, or target Mac.

## Historical release evidence

Apple accepted submission `b09b2947-4116-4eec-8c0a-0fea6946ddda`, and the project recorded it as stapled for an earlier local `0.1.0` candidate. That record does not prove that the same artifacts are still present or that a rebuilt candidate has the same notarization state. The generated section below is authoritative for the files currently available in the checkout.

<!-- MVP_RELEASE_NOTES_START -->
## Current Release Candidate

Generated: 2026-08-18T18:59:33.350Z

Repository: `aindaco1/social`
Source state: pending final commit/tag; generated from local worktree with uncommitted changes
Release state: complete local Apple Silicon artifact set present; verify signing, notarization, stapling, and smoke launch before publication.

## Artifacts

- Apple Silicon DMG: `src-tauri/target/release/bundle/dmg/Dust Wave Social_0.1.0_aarch64.dmg` (45 MB, SHA-256 `87b737941272423a942d9237997c0574f23ee09ef80c63f6a9713af9a9e24633`)
- Recorded notarization submission (verify it matches this DMG): `b09b2947-4116-4eec-8c0a-0fea6946ddda`
- Tauri updater latest.json: `src-tauri/target/release/bundle/latest.json` (702 B)
- Tauri updater archive: `src-tauri/target/release/bundle/macos/Dust Wave Social.app.tar.gz` (44 MB, SHA-256 `2cbcdd571e12f4022650e2acf97e52eff78f0e6e2d9ad10a35119dbf9870bfb8`)
- Tauri updater signature: `src-tauri/target/release/bundle/macos/Dust Wave Social.app.tar.gz.sig` (416 B)
- Updater version: `0.1.0`
- Updater URL: `https://github.com/aindaco1/social/releases/latest/download/Dust%20Wave%20Social.app.tar.gz`
- Updater signature embedded in latest.json: yes

## Readiness Snapshot

MVP readiness: 22 ready, 0 blocked, 13 manual.

Blocking issues:

- No blocking items were reported by the readiness script.

Manual acceptance still required:

- TikTok developer credential TIKTOK_CLIENT_KEY - from TikTok Developer Portal
- TikTok developer credential TIKTOK_CLIENT_SECRET - from TikTok Developer Portal
- Current release candidate notarization and stapling - submit the current DMG to Apple, wait for acceptance, staple it, and rerun strict artifact verification
- Media Staging token saved in desktop Services and Instagram local-media acceptance - requires launch Mac Keychain entry and live Instagram publish validation
- Local AI packaged-app offline model probe and reviewed output acceptance - requires signed/stapled app test with network disabled and operator review of generated derivatives
- X/Twitter live credential and publish acceptance - requires provider portal, live account, or separate target Mac
- Facebook/Meta live credential and Page acceptance - requires provider portal, live account, or separate target Mac
- Instagram live credential, publishing, scheduling, and insights acceptance - requires provider portal, live account, or separate target Mac
- Unsplash live credential acceptance - requires provider portal, live account, or separate target Mac
- Klipy production key and attribution acceptance - requires provider portal, live account, or separate target Mac
- Dust Wave account onboarding and live publish/import acceptance - requires provider portal, live account, or separate target Mac
- Updater higher-version draft release test - requires provider portal, live account, or separate target Mac
- Clean-Mac Gatekeeper install test - requires provider portal, live account, or separate target Mac

## Rollback Plan

1. Keep this signed/stapled DMG and the previous known-good DMG available before publishing the GitHub Release.
2. If a bad release is detected, pause scheduled publishing by quitting Dust Wave Social on affected Macs.
3. Preserve support logs and create a backup from System before uninstalling or downgrading.
4. Remove the bad GitHub Release assets or mark the release as draft so updater clients stop discovering it.
5. Publish or restore the last known-good `latest.json`, updater archive, updater signature, and DMG assets.
6. Install the previous known-good DMG on affected Macs and verify Gatekeeper opens it.
7. Reopen Dust Wave Social, verify app data loads, and reconnect provider accounts only if keychain credentials were intentionally removed.
8. If the bad release changed local data shape, restore from the last known-good Dust Wave backup instead of manually editing SQLite.
9. If provider tokens or broker credentials may be compromised, revoke them at the provider or broker before reconnecting accounts.
10. Record the incident, owner, customer impact, mitigation, and ship/no-ship decision in the release notes or private issue tracker.

## Publish Checklist

- Run `npm run desktop:release:artifact-check -- --require-updater --require-stapled`.
- Run `npm run desktop:smoke:launch`.
- Run `npm run mvp:launch:readiness` and confirm only expected manual acceptance items remain.
- Complete live provider credential/account acceptance.
- Complete clean-Mac Gatekeeper install.
- Complete a higher-version updater draft release test.
- Finalize provider, backup, support, and release owners.
- Publish only signed, stapled, checksum-recorded artifacts.
<!-- MVP_RELEASE_NOTES_END -->

## Remaining critical path

Complete these in order:

1. Produce or recover a complete signed release candidate and regenerate the section above.
2. Install the stapled DMG on a clean Apple Silicon Mac.
3. Configure production provider/media services without copying secrets into documentation.
4. Inventory and connect every Dust Wave account in MVP scope.
5. Run live publishing, scheduling, imports, reports, failure recovery, and provider-limit acceptance.
6. Run packaged offline Local AI Media acceptance and review derivative quality.
7. Test backup/restore and support-export redaction on clean app data.
8. Publish and install a signed higher-version updater test.
9. Complete visual, product-risk, security, ownership, and go/no-go review.
10. Tag and publish only the accepted candidate while retaining rollback artifacts.

## 1. Build and preserve the candidate

Before building:

```sh
npm ci
npm run desktop:release:preflight
npm run mvp:launch:readiness
npm run local-ai:models:check
npm run desktop:release:check
```

Build the signed Apple Silicon app, LGPL media sidecars, DMG, and updater artifacts:

```sh
npm run desktop:release:build:notarized:with-media-and-updater
npm run desktop:macos:notarize
npm run desktop:release:artifact-check -- --require-updater --require-stapled
npm run desktop:smoke:launch
npm run mvp:release:notes
npm run mvp:release:notes:check
```

Expected release files:

- Stapled `.app` in `src-tauri/target/release/bundle/macos/`.
- Stapled `.dmg` with an `/Applications` link and volume icon in `src-tauri/target/release/bundle/dmg/`.
- `latest.json` in `src-tauri/target/release/bundle/`.
- `.app.tar.gz` and `.app.tar.gz.sig` in `src-tauri/target/release/bundle/macos/`.

Do not run `npm run desktop:clean` while the candidate is under acceptance or notarization. Release artifacts are not committed; preserve the accepted candidate and a previous known-good installer outside disposable build storage before publishing.

The signed macOS release wrapper uses `~/Library/Caches/DustWaveSocial/target` through `src-tauri/target` unless `DUSTWAVE_RELEASE_USE_PROJECT_TARGET=true`. Before a release check, confirm `src-tauri/target` is absent or resolves to a writable directory. A broken symlink prevents Cargo and artifact checks from running.

## 2. Clean-Mac install

Use a separate clean Apple Silicon Mac when possible:

1. Transfer the checksum-recorded stapled DMG.
2. Open the DMG and drag Dust Wave Social into `/Applications`.
3. Launch from `/Applications` and confirm there is no Gatekeeper warning.
4. Confirm first launch works without existing Dust Wave app data.
5. Quit and relaunch to test the existing-data path.
6. Record the exact macOS dialog and stop if installation or launch fails.

Intel and universal macOS builds are outside MVP scope.

## 3. Configure services

In Services, use `Copy All Setup` or `Copy Missing`. The packet must contain callback URLs, scopes, and setup instructions but no existing secret values.

Store credentials only in provider portals, Cloudflare/GitHub secret stores, or the app's Keychain-backed forms.

### X/Twitter

- Callback: `http://localhost/callback`.
- Scopes: `tweet.read tweet.write users.read offline.access`.
- Desktop credentials: API Key and API Secret.
- Confirm the configured tier supports required posting, media upload, reads/imports, and rate limits.

### Meta, Facebook Pages, and Instagram

- Shared callback: `http://localhost/callback`.
- Shared desktop credentials: Meta App ID and App Secret.
- Current default Graph API version: `v25.0`.
- Required permissions depend on approved workflows and currently include `business_management`, `pages_show_list`, `read_insights`, `pages_manage_posts`, `pages_read_engagement`, `pages_manage_engagement`, `instagram_basic`, `instagram_content_publish`, `instagram_manage_insights`, and `instagram_manage_comments` when comment management is used.
- Instagram accounts must be Business or Creator accounts connected to the required Meta/Page assets.
- Local Instagram images require an active Media Staging service using `https://dustwave-media-staging.jogo.workers.dev` and a matching Keychain token.

### TikTok

- Desktop values: Client Key, HTTPS broker URL, and Assisted publishing mode.
- Broker-only secret: TikTok Client Secret.
- Analytics scopes: `user.info.basic`, `user.info.stats`, and `video.list`.
- Broker callback: the deployed broker's `/api/tiktok/oauth/callback` URL.
- Direct publishing remains disabled until TikTok approves `video.upload` or `video.publish` and Dust Wave deliberately enables a stronger mode.

### Unsplash and Klipy

- Save the production Unsplash access key and confirm attribution/API access requirements.
- Save the production Klipy key, configure content filters/blocklists, and confirm current branding requirements.
- Klipy media remains a provider reference and temporary publish-time asset. It must not enter the reusable media library without written permission; see [GIF_PROVIDER_DECISION.md](GIF_PROVIDER_DECISION.md).

### Mastodon

Mastodon is configured per server from Accounts. Use the server host, not a profile URL, and confirm dynamic app registration is permitted on each target server.

Provider setup is complete only when the required service cards are active, `Copy Missing` reports no unexpected gaps, logs remain secret-free, and the setup packet still contains no credentials.

## 4. Onboard accounts

Use `Copy Intake CSV` in Accounts and record one row per managed account. Supported provider values are `twitter`, `facebook_page`, `instagram`, `mastodon`, and `tiktok`.

For every account:

1. Connect through the provider-specific Accounts flow.
2. Confirm the account is authorized.
3. Refresh account metadata where supported.
4. Queue/import history when required.
5. Confirm Reports receives the expected audience, post, insight, or video metrics.
6. Keep unsupported/manual workflows in the notes column rather than representing them as connected providers.

TikTok onboarding begins at the broker OAuth start URL. Paste only the broker-issued opaque connection credential into Dust Wave Social; never paste TikTok access/refresh tokens or the client secret.

## 5. Live provider acceptance

For each real account, exercise every supported combination needed by Dust Wave:

- Connect, refresh, disconnect/revoke, and reconnect.
- Publish text, image, video, GIF, or assisted content where supported.
- Schedule a future post and let the local app-open worker publish it.
- Confirm the final provider ID or URL is stored.
- Import account and post metrics and compare the report with the provider.
- Force or simulate expired credentials, provider rejection, and rate limiting.
- Confirm errors are actionable and redacted, deferred jobs stay scoped, and retries do not duplicate posts.
- Confirm X/Twitter blocks identical simultaneous posting to multiple X accounts.

Instagram-specific acceptance:

1. Publish a supported static image from local media.
2. Confirm the Worker stages it at a temporary public HTTPS URL.
3. Confirm Meta publishes it or returns a clear provider error.
4. Confirm the staged object is deleted after the attempt or by scheduled cleanup.
5. Verify professional-account rules, current media specifications, app-review gates, and provider posting limits.
6. Import and verify Instagram insights.

TikTok-specific acceptance:

1. Complete an assisted publishing test.
2. Import analytics through the broker.
3. Verify views, likes, comments, shares, audience data, and revoked-credential behavior.
4. Confirm direct API publishing remains unavailable in Assisted mode.

Current provider formats and deliberate exclusions are listed in [FEATURES.md](FEATURES.md).

## 6. Local AI Media acceptance

Follow the packaged offline procedure in [LOCAL_AI.md](LOCAL_AI.md). Acceptance requires the current signed app, Wi-Fi disabled, a successful runtime/model probe, representative upscaling, cancellation cleanup, original preservation, metadata verification, quality review, backup/restore, and redaction checks.

True embedding search and model-backed image captioning are deferred; do not describe the current profile-based helpers as those features.

## 7. Backup, restore, and support hygiene

Before live testing, create a backup from System:

1. Confirm it contains the database, app-owned media, derivatives, and manifest.
2. Restore to clean app data or a clean target Mac.
3. Confirm posts, media, reports, and local-AI derivative metadata return.
4. Confirm Keychain secrets are absent and reconnect accounts as needed.
5. Test a draft save, media preview, account refresh, and scheduled job before resuming operations.
6. Export logs and confirm tokens, API keys, client secrets, refresh tokens, and webhook secrets are absent.

Use [SUPPORT_RUNBOOK.md](SUPPORT_RUNBOOK.md) for failure and incident procedures.

## 8. Updater acceptance

A real updater test requires a visible release with a version higher than the installed candidate:

1. Commit and tag the accepted source.
2. Bump to a test version such as `0.1.1`.
3. Build, sign, notarize, and staple the new artifacts with the same updater private key.
4. Upload `latest.json`, `.app.tar.gz`, `.app.tar.gz.sig`, and the DMG to a GitHub Release.
5. From installed `0.1.0`, check for updates in System.
6. Download, install, and relaunch.
7. Confirm the version changed and app data survived.

Losing or replacing the updater private key prevents installed clients from trusting future updates. Back it up outside the repository.

## 9. Final go/no-go

Before publication, confirm:

- The generated release-candidate section names a complete, current artifact set.
- Strict artifact verification and packaged smoke launch pass.
- Clean-Mac install and relaunch pass.
- Required services and accounts are configured, connected, refreshed, and imported.
- Live publishing, scheduling, reports, and failure recovery pass.
- Packaged Local AI Media acceptance passes.
- Backup/restore, log redaction, and support procedures pass.
- The higher-version updater test passes.
- Visual QA passes at 1024px, 1280px, and wide desktop widths against current Pool/Store design direction.
- Gambado font redistribution rights are confirmed before broader public distribution.
- The review in [BEST_PRACTICES.md](BEST_PRACTICES.md) has owners, mitigations, and ship/no-ship decisions for every red flag.
- Owners are named for provider credentials, Apple credentials, updater hosting, backups, support, incident response, and final release approval.
- A previous known-good DMG and updater set are preserved for rollback.

Then regenerate and verify the release section:

```sh
npm run mvp:launch:readiness
npm run mvp:release:notes
npm run mvp:release:notes:check
npm run desktop:release:artifact-check -- --require-updater --require-stapled
npm run desktop:smoke:launch
```

Publish only signed, notarized, stapled, checksum-recorded artifacts from the accepted source tag.

## Release engineering reference

### CI

- `.github/workflows/desktop.yml` runs release checks on branch changes. A synchronized `vX.Y.Z` tag builds the exact tagged source, signs and notarizes the app, creates and notarizes the DMG, verifies app/DMG/updater identity, publishes the GitHub Release, and smokes the published updater. Manual dispatch exposes the same release path.
- `.github/workflows/tiktok-broker.yml` tests and can deploy the TikTok broker and D1 migrations.
- `.github/workflows/media-staging.yml` tests, dry-runs, and can deploy the media-staging Worker.
- `.github/workflows/run-tests.yml` covers the retained Mixpost PHP package.

### Apple and updater credentials

Local release automation reads signing, notarization, and updater material from the sibling `Apple Auth` folder or environment variables. Never commit those files.

Generate updater signing material once with:

```sh
npm run desktop:updater:keys
```

The release repository is `aindaco1/social`. Each updater manifest uses the exact matching `vX.Y.Z` release asset URL; the app discovers that manifest through GitHub's latest-release endpoint. Keep the updater private key stable across releases.

### FFmpeg and FFprobe

The MVP ships Apple Silicon LGPL-only sidecars built from approved source:

```sh
npm run desktop:media:build-lgpl
```

Do not ship arbitrary Homebrew binaries or builds that report GPL/nonfree flags. Keep versions, hashes, configure flags, source details, and license output in `../THIRD_PARTY_NOTICES.md`.

### Repository hygiene

Do not commit build outputs, generated signing/updater overlays, Worker `.dev.vars`, provider credentials, Apple credentials, local app data, or notarized artifacts. Review `git status` before tagging. Decide when the retained Mixpost package can be archived only after live-provider acceptance and rollback confidence are complete.
