# Dust Wave Social MVP Launch Plan

Updated: 2026-08-22

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

## Published release evidence

[Dust Wave Social v0.1.6](https://github.com/aindaco1/social/releases/tag/v0.1.6) was published on 2026-08-22 from commit `c13213832455b3f33f137af57a801a074b633ada` for Apple Silicon macOS.

- Apple accepted app submission `16f864be-ec6a-40a8-8853-29d918d8a2ef`.
- Apple accepted DMG submission `ecd8851d-ea24-4b66-9a66-486c2d41a801`; stapler validation and Gatekeeper assessment passed.
- Published DMG SHA-256: `ecc9dd2800a7caeb32f147541500e247ba3426262bc2ae9a98de0ba503e6cf47`.
- Published updater archive SHA-256: `b6b36f6a164a93defc2e37485971e660b2f91a62a2174b981302b22c14342cec`.
- Published updater signature SHA-256: `2330537a91b1b25a3337250fc0b890467505a4a2595c0b7f68a9b21dc0012b7f`.
- Published app ZIP SHA-256: `3b364adf693db25a4f41e28c3100fb5b20cc4449a90e751ad4e6d2e62c13d160`.
- Published `latest.json` SHA-256: `79585cd8ad9902ee755435ad803e9cb4ba2b420fed944f13cb09fba99830a75f`.
- The protected [v0.1.6 tag workflow](https://github.com/aindaco1/social/actions/runs/32588493376) passed release checks, signing, app and DMG notarization/stapling, strict artifact verification, publication, and public manifest verification.
- The protected updater smoke downloaded the public v0.1.5 app, installed all 45,594,398 signed updater bytes, verified the staged bundle changed to v0.1.6, and proved the automatic process hop from PID 31354 to PID 31710.
- An independent post-publication download reproduced all five GitHub asset digests, matched the live manifest and updater signature asset, validated the stapled DMG ticket and `/Applications` layout, passed Gatekeeper and disk-image verification, and verified the mounted, updater-archive, and app-ZIP bundles' code signatures, stapled tickets, bundle identity, arm64 architecture, and v0.1.6 version.

The first 0.1.1 candidate passed signing, notarization, artifact checks, and updater installation, but its workflow incorrectly checked the untouched source app after updating a canonical staged copy. The fail-closed workflow returned that candidate to draft. Version 0.1.2 moved the version assertion into the staged-app harness and published only after the corrected hop passed. A second local packaged-app download smoke timed out on the preserved local 0.1.0 build even though the same archive downloaded directly in 1.55 seconds. Version 0.1.5 is the immediate rollback baseline for v0.1.6; version 0.1.0 remains the earliest archived known-good release.

Hands-on testing then found that versions 0.1.0 through 0.1.2 stored Tauri's updater resource in a deep Vue `ref`. Vue proxied the resource, so Tauri could not read its private resource ID and installation failed before download. Version 0.1.3 changes that state to `shallowRef` and adds a regression test against the real Tauri `Update` class. Because affected clients cannot install the fix in-app, operators must install the 0.1.3 or newer DMG over the existing app once.

The first hands-on 0.1.3 to 0.1.4 hop downloaded and installed the signed release, but the old process remained on “Installing update” after its bundle had been replaced. Version 0.1.5 moved download, verification, installation, and restart into one Rust-side operation so a WebView response cannot strand future handoffs. Because 0.1.4 cannot execute code that only exists in the replacement bundle, the 0.1.4 to 0.1.5 bridge still requires one manual quit/reopen after installation. The v0.1.6 protected smoke has now proved the automatic process hop from v0.1.5. Hands-on app-data and Connections > Provider setup acceptance remain operator checks.

The generated section below describes local checkout artifacts, which may differ from the published production files above.

<!-- MVP_RELEASE_NOTES_START -->
## Current Local Release Artifacts

Generated: not generated; no local DMG

Repository: `aindaco1/social`
Source state: generated from local worktree with uncommitted changes
Release state: no complete local release candidate; recover or rebuild the missing artifacts before acceptance or publication.

## Artifacts

- Apple Silicon DMG: missing at `src-tauri/target/release/bundle/dmg/Dust Wave Social_0.1.7_aarch64.dmg`
- Recorded notarization submission (verify it matches this DMG): `ecd8851d-ea24-4b66-9a66-486c2d41a801`
- Tauri updater latest.json: missing at `src-tauri/target/release/bundle/latest.json`
- Tauri updater archive: missing at `src-tauri/target/release/bundle/macos/Dust Wave Social.app.tar.gz`
- Tauri updater signature: missing at `src-tauri/target/release/bundle/macos/Dust Wave Social.app.tar.gz.sig`
- Updater version: `0.1.7`
- Updater URL: not generated
- Updater signature embedded in latest.json: no

## Readiness Snapshot

MVP readiness: 21 ready, 1 blocked, 13 manual.

Blocking issues:

- Local release artifact set - DMG, latest.json, updater archive, and updater signature under src-tauri/target/release/bundle

Manual acceptance still required:

- TikTok developer credential TIKTOK_CLIENT_KEY - from TikTok Developer Portal
- TikTok developer credential TIKTOK_CLIENT_SECRET - from TikTok Developer Portal
- Current release candidate notarization and stapling - submit the current DMG to Apple, wait for acceptance, staple it, and rerun strict artifact verification
- Instagram Local Media paired in Connections > Provider setup and local-image acceptance - requires launch Mac Keychain entry and live Instagram publish validation
- Local AI packaged-app offline model probe and reviewed output acceptance - requires signed/stapled app test with network disabled and operator review of generated derivatives
- X/Twitter live credential and publish acceptance - requires provider portal, live account, or separate target Mac
- Facebook/Meta live credential and Page acceptance - requires provider portal, live account, or separate target Mac
- Instagram live credential, publishing, scheduling, and insights acceptance - requires provider portal, live account, or separate target Mac
- Unsplash live credential acceptance - requires provider portal, live account, or separate target Mac
- Klipy production key and attribution acceptance - requires provider portal, live account, or separate target Mac
- Dust Wave account onboarding and live publish/import acceptance - requires provider portal, live account, or separate target Mac
- Clean-Mac Gatekeeper install test - requires provider portal, live account, or separate target Mac
- Operator updater installation, relaunch, and app-data acceptance - requires the installed previous public release and representative app data on the target Mac

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
- Complete operator updater installation, relaunch, and app-data acceptance from the installed rollback version.
- Finalize provider, backup, support, and release owners.
- Publish only signed, stapled, checksum-recorded artifacts.
<!-- MVP_RELEASE_NOTES_END -->

## Remaining critical path

Complete these in order:

1. Preserve the published v0.1.6 DMG and updater assets with v0.1.5 as the rollback baseline.
2. Install the stapled DMG on an independent clean Apple Silicon Mac.
3. Configure production provider/media services without copying secrets into documentation.
4. Inventory and connect every Dust Wave account in MVP scope.
5. Run live publishing, scheduling, imports, reports, failure recovery, and provider-limit acceptance.
6. Run packaged offline Local AI Media acceptance and review derivative quality.
7. Test backup/restore and support-export redaction on clean app data.
8. Start from an installed v0.1.5 app with representative data, update to v0.1.6, and confirm automatic relaunch plus app-data and Keychain-backed configuration survival.
9. Complete visual, product-risk, security, ownership, and operational go/no-go review.

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

In Connections > Provider setup, use `Copy All Setup` or `Copy Missing`. The packet must contain callback URLs, scopes, and setup instructions but no existing secret values.

Store credentials only in provider portals, Cloudflare/GitHub secret stores, or the app's Keychain-backed forms.

Each provider has one `Save [Provider] Settings` action. It saves every newly entered credential to macOS Keychain, saves the visible configuration and Active state together, and reports the result in the provider card. Leave an already available credential blank to keep its current value. Activating a provider is blocked until every required credential is either already available or entered in the form.

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
- Local Instagram images require an active Instagram Local Media service using `https://dustwave-media-staging.jogo.workers.dev` and a paired device credential in Keychain. Users pair with a short-lived, one-use code and never need Cloudflare or the reusable operator token.

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

Mastodon is configured per server from Connections > Connected accounts. Use the server host, not a profile URL, and confirm dynamic app registration is permitted on each target server.

Provider setup is complete only when the required service cards are active, `Copy Missing` reports no unexpected gaps, logs remain secret-free, and the setup packet still contains no credentials.

## 4. Onboard accounts

Use `Copy Intake CSV` in Connections > Connected accounts and record one row per managed account. Supported provider values are `twitter`, `facebook_page`, `instagram`, `mastodon`, and `tiktok`.

For every account:

1. Connect through the provider-specific Add Account flow in Connections > Connected accounts.
2. Confirm the account is authorized.
3. Refresh account metadata where supported.
4. Queue/import history when required.
5. Confirm Analytics receives the expected audience, post, insight, or video metrics.
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

The protected v0.1.6 tag workflow passed public manifest resolution, downloaded the published v0.1.5 app archive, installed the signed v0.1.6 update into a canonical staged copy, verified that copy's bundle version, and proved the automatic process relaunch. The release uses the same updater private key trusted by earlier versions. This closes automated signed-updater acceptance but does not substitute for verifying representative operator data and Keychain-backed setup on the installed app.

Operator acceptance remains:

1. Back up representative app data from System.
2. Confirm the installed v0.1.5 app loads that data before starting the update.
3. From v0.1.5, use the top-right Update action or the detailed controls in System to download and install v0.1.6.
4. Confirm the old process exits, Dust Wave Social relaunches automatically, and System reports v0.1.6.
5. Confirm representative app data, Keychain-backed service readiness, and saved Provider setup configuration survived.

Losing or replacing the updater private key prevents installed clients from trusting future updates. Back it up outside the repository.

## 9. Final go/no-go

Before treating the current release as operationally launch-ready, or publishing a later release, confirm:

- The generated local-artifact section names a complete, current artifact set when building a later release.
- Strict artifact verification and packaged smoke launch pass.
- Clean-Mac install and relaunch pass.
- Required services and accounts are configured, connected, refreshed, and imported.
- Live publishing, scheduling, reports, and failure recovery pass.
- Packaged Local AI Media acceptance passes.
- Backup/restore, log redaction, and support procedures pass.
- Operator updater installation, relaunch, and app-data acceptance pass on the target Mac.
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
