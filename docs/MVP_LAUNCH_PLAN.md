# Dust Wave Social MVP Launch Plan

Updated: 2026-07-16

Audience: Dust Wave operators preparing the Apple Silicon macOS MVP release.

This plan is the single canonical MVP launch document. It consolidates the previous release checklist, release notes, finish-line checklist, and MVP launch plan after the Tauri/Rust desktop migration, Pool/Store design pass, Mixpost parity work, bundled LGPL-only FFmpeg/FFprobe sidecars, signing wrapper, updater scaffolding, and CI hardening.

## MVP Launch Definition

The MVP is ready to launch when Dust Wave can install a notarized Apple Silicon macOS app, connect production social/media provider credentials, onboard Dust Wave social accounts, publish and schedule representative posts across X, Facebook Pages, Instagram, Mastodon, and TikTok-assisted workflows, import supported account analytics including TikTok broker analytics and Instagram insights, run local-only AI media tools behind a Labs flag, restore from backup, and recover from provider/auth failures without using development-only panels or raw database access.

## Current Status

- Green: desktop release checks, Rust tests, CI Desktop workflow, legacy PHP test workflow, asset workflow, signed local artifacts, bundled media sidecars, updater artifact generation, signed/stapled DMG verification, packaged app smoke launch, TikTok broker D1 database, TikTok broker initial migration, and deployed TikTok broker health endpoint.
- Green: `npm run mvp:launch:readiness` now separates ready code/infrastructure from manual launch acceptance without exposing secret values.
- Yellow: production provider credentials and live account acceptance are not complete.
- Yellow: TikTok developer portal Client Key/Secret and live TikTok analytics acceptance are not complete.
- Yellow: Instagram is now explicit MVP scope. First-class account connection, publishing, import, reporting, UI, and local-media staging code are implemented; live Meta credential/account acceptance is still required.
- Green: Cloudflare R2 media staging bucket, Worker, GitHub secrets/variables, and live smoke test are complete at `https://dustwave-media-staging.jogo.workers.dev`.
- Yellow: LiteRT.js local AI media is now MVP scope behind a Labs flag. The app bundles LiteRT Wasm, bundles a checksum-validated Real-ESRGAN-x4plus TFLite upscaling model, probes WebGPU/Wasm, runs tiled model-backed x4 upscaling locally with progress/cancel states, and ships original-preserving media tools; packaged offline behavior acceptance and output quality review are still required.
- Green: MVP release notes and rollback instructions are generated from the current artifacts with `npm run mvp:release:notes`.
- Yellow: updater publish/install test still needs a higher-version draft release.
- Red until completed: clean-Mac Gatekeeper install, live provider publishing validation, and final launch go/no-go.

<!-- MVP_RELEASE_NOTES_START -->
## Current Release Candidate

Generated: 2026-07-15T15:45:57.861Z

Repository: `aindaco1/social`
Source state: pending final commit/tag; generated from local worktree with uncommitted changes
Release state: signed and notarized local Apple Silicon candidate; public GitHub Release still requires operator approval.

## Artifacts

- Stapled Apple Silicon DMG: `src-tauri/target/release/bundle/dmg/Dust Wave Social_0.1.0_aarch64.dmg` (45 MB, SHA-256 `51eb249242318a9ada34413f3a1f3adef872b7aa35259ea678a51953e8da315f`)
- Apple notarization submission: `b09b2947-4116-4eec-8c0a-0fea6946ddda`
- Tauri updater latest.json: `src-tauri/target/release/bundle/latest.json` (702 B)
- Tauri updater archive: `src-tauri/target/release/bundle/macos/Dust Wave Social.app.tar.gz` (44 MB, SHA-256 `266f3793d20e6b34e8d2b84539e40359eeec38970f78fe6ae52095818f9dd50c`)
- Tauri updater signature: `src-tauri/target/release/bundle/macos/Dust Wave Social.app.tar.gz.sig` (416 B)
- Updater version: `0.1.0`
- Updater URL: `https://github.com/aindaco1/social/releases/latest/download/Dust%20Wave%20Social.app.tar.gz`
- Updater signature embedded in latest.json: yes

## MVP Scope

- Apple Silicon macOS desktop app for managing Dust Wave social accounts.
- First-class account, publishing, scheduling, import, reporting, and failure-state flows for X/Twitter, Facebook Pages, Instagram, Mastodon, and TikTok-assisted workflows where credentials and provider approvals allow.
- Cloudflare R2 media staging for Instagram local-image publishing, with temporary public HTTPS URLs and scheduled expired-object cleanup.
- TikTok broker-backed analytics scaffold with the desktop app storing only broker-safe account credentials.
- Local media library, Mixpost-parity post workflows, reports, backup/restore, desktop notifications, and support exports.
- Local AI Media Labs behind a setting: bundled LiteRT.js runtime, bundled model weights, model-backed upscaling derivatives, media preflight, smart crops, local media search, and review-required alt-text drafts.

## Important Limits

- Provider portals, production credentials, provider app review, live posting acceptance, and real account onboarding are still manual.
- TikTok direct API publishing remains approval-gated; MVP uses assisted publishing unless TikTok approves stronger publishing scopes.
- Instagram requires professional Business or Creator accounts connected to the required Meta/Page assets.
- Apple Silicon macOS is the MVP target. Intel/universal macOS builds are out of scope.
- Klipy GIFs are provider references and transient publish-time assets unless Klipy grants written permission for permanent media-library imports.
- Gambado font redistribution rights still need final human confirmation before broader public distribution.

## Readiness Snapshot

MVP readiness: 21 ready, 0 blocked, 12 manual.

Manual acceptance still required:

- TikTok developer credential TIKTOK_CLIENT_KEY - from TikTok Developer Portal
- TikTok developer credential TIKTOK_CLIENT_SECRET - from TikTok Developer Portal
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



## Critical Path

1. Install the notarized DMG on a clean Apple Silicon Mac and confirm first launch.
2. Configure provider credentials in Services.
3. Onboard Dust Wave accounts in Accounts.
4. Validate local AI media tools in the packaged app.
5. Validate live publishing, scheduling, media, Instagram insights imports, TikTok broker analytics imports, reports, and provider failure handling.
6. Publish a test update and define rollback.
7. Run ethical/security acceptance and final visual QA.
8. Regenerate release notes, tag the release, publish notarized artifacts, and keep rollback artifacts available.

Run this whenever launch state changes:

```sh
npm run mvp:launch:readiness
npm run mvp:release:notes
npm run mvp:release:notes:check
```

## Manual Launch Process From Here

This is the operator sequence for the remaining manual input. It assumes the code and infrastructure state in the readiness snapshot is still current.

### 1. Preserve The Release Candidate

Do not run `npm run desktop:clean` until launch testing is finished. Keep these launch artifacts available:

```sh
src-tauri/target/release/bundle/dmg/Dust Wave Social_0.1.0_aarch64.dmg
src-tauri/target/release/bundle/latest.json
src-tauri/target/release/bundle/macos/Dust Wave Social.app.tar.gz
src-tauri/target/release/bundle/macos/Dust Wave Social.app.tar.gz.sig
```

Before each major launch step, run:

```sh
npm run mvp:launch:readiness
npm run desktop:release:preflight
npm run mvp:release:notes
npm run mvp:release:notes:check
```

If the DMG, updater archive, signature, or `latest.json` changes, regenerate this document with `npm run mvp:release:notes` before sharing release notes or publishing artifacts.

### 2. Clean-Mac Install

Use a separate clean Apple Silicon Mac if possible.

1. Transfer the signed/stapled DMG listed in the release-candidate section.
2. Open the DMG in Finder.
3. Drag Dust Wave Social into `/Applications`.
4. Launch Dust Wave Social from `/Applications`.
5. Confirm macOS opens it without Gatekeeper warnings.
6. Confirm first launch works with no existing Dust Wave app data.
7. Quit and relaunch to confirm startup works with an existing app-data directory.
8. If the app fails to open, capture the exact macOS dialog and do not rebuild until the failure is understood.

### 3. Configure Services

Open Dust Wave Social, go to Services, and click `Copy All Setup`. Paste the packet into a private working document. The packet is intended to guide provider setup and should not include secret values.

Store credentials only through the app's Services forms, the provider portal, Cloudflare secrets, GitHub secrets, or another approved secret store. Do not paste provider secrets into docs, screenshots, support exports, issue trackers, or chat.

Configure the MVP services:

- X/Twitter: callback `http://localhost/callback`; scopes `tweet.read tweet.write users.read offline.access`; save API Key and API Secret in Services.
- Meta/Facebook/Instagram: callback `http://localhost/callback`; use the shared Meta App ID/App Secret; confirm Facebook Page and Instagram permissions.
- Media Staging: base URL `https://dustwave-media-staging.jogo.workers.dev`; save the staging token in Services and set the service Active.
- TikTok: desktop Services gets the Client Key and broker URL; the TikTok client secret stays only in Cloudflare/GitHub secrets for the broker.
- Unsplash: save the access key in Services and confirm attribution/API access requirements.
- Klipy: save the API key in Services and confirm attribution, content filters, and no permanent provider GIF storage.
- Mastodon: configure per server/account from Accounts, not as one global Services credential.

After TikTok Developer Portal values are available, set them in the broker without printing values:

```sh
npm run tiktok:broker:config:check
npx wrangler secret put TIKTOK_CLIENT_KEY --config workers/tiktok-broker/wrangler.generated.jsonc
npx wrangler secret put TIKTOK_CLIENT_SECRET --config workers/tiktok-broker/wrangler.generated.jsonc
npm run tiktok:broker:migrate
npm run tiktok:broker:deploy
```

Set matching GitHub secrets for CI deployment if the broker should be deployable from GitHub Actions:

```sh
gh secret set TIKTOK_CLIENT_KEY --repo aindaco1/social
gh secret set TIKTOK_CLIENT_SECRET --repo aindaco1/social
```

### 4. Onboard Accounts

In Accounts, click `Copy Intake CSV` and fill one row for every Dust Wave account that should be managed.

Use provider values `twitter`, `facebook_page`, `instagram`, `mastodon`, or `tiktok`. Include Instagram Business/Creator accounts as explicit `instagram` rows, not as notes under Facebook Pages.

For each account:

1. Connect it through Accounts.
2. Confirm the account card appears authorized.
3. Refresh the account.
4. Queue/import history when `import_history` is `yes`.
5. Confirm Reports populate with the expected account metrics, post metrics, audience data, provider URLs, or remote IDs.

Instagram accounts must be Business or Creator accounts connected to the required Meta/Page assets.

For TikTok accounts, open the broker OAuth start URL, authorize the account, copy the broker-issued connection credential, and paste it into the TikTok account form. TikTok analytics acceptance requires `user.info.basic`, `user.info.stats`, and `video.list`.

### 5. Live Publishing Acceptance

For each real account in MVP scope:

1. Publish a text post where supported.
2. Publish an image post where supported.
3. Publish a video post where supported.
4. Publish GIF/provider media where supported.
5. Schedule a future post and let the local worker publish it.
6. Confirm the final provider URL or remote ID is stored.
7. Revoke one token or broker connection and verify the app surfaces failure/reconnect behavior clearly.
8. Force or simulate rate-limit/provider errors where practical and verify jobs defer, retry, or fail with operator-readable messages.
9. Confirm X/Twitter refuses simultaneous posting to multiple X accounts.

Instagram-specific acceptance:

1. Publish from local desktop image media.
2. Confirm the Cloudflare R2 staged media URL is created.
3. Confirm the publish succeeds or returns a clear Meta provider error.
4. Confirm the staged object expires, is deleted after publish, or is removed by the hourly Worker cleanup.
5. Confirm Instagram-specific media specs, professional-account requirements, app-review gates, and daily publishing limits are surfaced clearly.

TikTok-specific acceptance:

1. Complete one assisted publishing test.
2. Import analytics through the broker.
3. Confirm views, likes, comments, shares, and audience data appear in reports.
4. Confirm direct API publishing remains blocked unless TikTok has approved stronger publishing scopes and the stronger publishing mode is intentionally enabled.

### 6. Local AI Acceptance

Use the installed packaged app, not a dev server.

1. Enable Local AI Media Labs in Settings.
2. Disable Wi-Fi for the offline acceptance pass.
3. Run the LiteRT capability probe.
4. Run model-backed Upscale on representative images.
5. Confirm original media remains unchanged.
6. Confirm derivatives include source media, model/runtime, dimensions, and SHA-256 metadata.
7. Test media quality preflight, smart crop suggestions, local semantic media search, and alt-text drafting.
8. Confirm alt-text drafts remain visibly generated and editable before publishing.
9. Review generated image quality manually before accepting the feature for MVP.

### 7. Backup, Restore, And Support Hygiene

Before live testing, create a backup from System.

1. Confirm the backup includes database, app-owned media, and manifest.
2. Restore into a clean app-data directory or clean target Mac.
3. Confirm posts, media, reports, local AI derivatives, and metadata return.
4. Confirm OS keychain secrets are excluded.
5. Reconnect provider accounts after restore.
6. Export logs and confirm no access tokens, refresh tokens, client secrets, API keys, or webhook secrets appear.
7. Confirm Copy App Data Path gives support the right local folder without exposing raw database UI.

### 8. Updater Test

The app uses GitHub Releases for updater discovery. A real updater test needs a higher-version release asset that the installed app can see.

Recommended sequence:

1. Commit and tag the current source first.
2. Bump the app version to a test version such as `0.1.1`.
3. Build signed media/updater artifacts.
4. Notarize and staple the DMG.
5. Upload `latest.json`, `.app.tar.gz`, `.app.tar.gz.sig`, and the DMG to a visible GitHub Release.
6. In installed `0.1.0`, go to System and check for updates.
7. Download, install, and relaunch the update.
8. Confirm the version changed and app data survived.

Updater clients cannot accept updates signed with a different updater private key. Keep the updater private key backed up outside the repository.

### 9. Final Go/No-Go

Before launch, run:

```sh
npm run desktop:release:artifact-check -- --require-updater --require-stapled
npm run desktop:smoke:launch
npm run mvp:launch:readiness
npm run mvp:release:notes
npm run mvp:release:notes:check
```

Confirm:

- Provider credentials are complete.
- All MVP accounts are connected, refreshed, and imported where needed.
- Live publishing tests passed.
- Backup/restore passed.
- Local AI packaged-app acceptance passed.
- Clean-Mac install passed.
- Updater higher-version test passed.
- Release notes, rollback plan, and owners are reviewed.
- Only signed, stapled, checksum-recorded artifacts will be published.

## P0 1: Apple Notarization

Status: accepted and stapled.

Apple accepted submission `b09b2947-4116-4eec-8c0a-0fea6946ddda` for the current signed media/updater DMG. Strict artifact verification and packaged smoke launch passed. Current DMG SHA-256: `51eb249242318a9ada34413f3a1f3adef872b7aa35259ea678a51953e8da315f`.

```sh
npm run desktop:release:artifact-check -- --require-updater --require-stapled
npm run desktop:smoke:launch
```

Install the stapled DMG on a clean Apple Silicon Mac and confirm Gatekeeper opens it without warnings.

## P0 2: Provider Setup And Credentials

Goal: every MVP service needed by Dust Wave is configured, active, and redaction-safe before account onboarding begins.

Do not paste provider secrets into docs, issue trackers, screenshots, setup packets, onboarding packets, or chat. Store secrets only through the app's Services forms or a secure secret store.

### Service Packet

1. Open Dust Wave Social.
2. Go to Services.
3. Use Copy All Setup for a full provider setup packet, or Copy Missing if some services are already active.
4. Paste the packet into private working notes that can be edited during setup.
5. Use the packet's Create App URLs, callback URLs, scopes, and setup values when creating provider apps.
6. Keep the packet as a non-secret checklist. It intentionally does not include existing client secrets, API keys, access tokens, or refresh tokens.

### X/Twitter

1. Open the X developer portal from Services or go to `https://developer.twitter.com/en/portal/projects-and-apps`.
2. Create or update the Dust Wave app.
3. Configure the OAuth callback URL as `http://localhost/callback`.
4. Enable the app scopes used by Dust Wave: `tweet.read tweet.write users.read offline.access`.
5. Confirm the app access tier is compatible with posting, media upload, reads/imports, and rate limits. The app defaults to the `pay_as_you_go` tier in Dust Wave.
6. Copy the X app values into Services:
   - API Key
   - API Secret
7. Save each credential field.
8. Set the X service Active.
9. Save Service.
10. Confirm the service card says configured and active.

### Meta App For Facebook Pages And Instagram

1. Open Meta Developers from Services or go to `https://developers.facebook.com/apps`.
2. Create or update the Dust Wave app using the Pages and Instagram professional-account use cases.
3. Configure the OAuth callback URL as `http://localhost/callback`.
4. Configure the default Graph API version. Dust Wave defaults to `v25.0`.
5. Request or confirm the permissions needed for MVP Facebook Page publishing/reporting and Instagram publishing/insights:
   - `business_management`
   - `pages_show_list`
   - `read_insights`
   - `pages_manage_posts`
   - `pages_read_engagement`
   - `pages_manage_engagement`
   - `instagram_basic`
   - `instagram_content_publish`
   - `instagram_manage_insights`
   - `instagram_manage_comments`
6. Complete Meta app review or role-based tester/admin setup as needed for Dust Wave's managed Pages and Instagram professional accounts.
7. Copy the Meta app values into Services:
   - App ID
   - App Secret
8. Save each credential field.
9. Set the Facebook service Active.
10. Save Service.
11. Confirm the service card says configured and active.

### Instagram

Instagram is MVP scope as a first-class platform, not just a permission bundle on the Facebook Page flow. Meta's Instagram APIs are for Instagram professional accounts, meaning Business or Creator accounts. The content-publishing API has provider-side posting limits and media rules, so acceptance must validate the real Dust Wave accounts, not only mocked provider tests.

Implementation status: first-class `instagram` provider discovery, connection, refresh, publishing/scheduling, import, reporting, and desktop UI paths are implemented. Local desktop media is staged through the Media Staging service before Instagram publishing because Meta requires a public HTTPS media URL.

Manual setup:

1. Confirm each Dust Wave Instagram account is a Business or Creator account.
2. Confirm each Instagram account is connected to the appropriate Facebook Page or Meta business asset required by the selected Meta login/API flow.
3. Confirm the Meta app has the Instagram API product enabled and the permissions listed above approved or available to tester/admin roles.
4. Confirm the app review submission includes screen recordings for Instagram login, account selection, publishing, insights import, and comment-management if `instagram_manage_comments` is used.
5. Validate the provider's current media specifications and daily API-published post limit before launch.
6. In Services, keep using the shared Meta App ID/App Secret. Do not create a second unrelated desktop secret store for Instagram unless the implementation intentionally separates Meta products.
7. In Services, save the Media Staging token, confirm the HTTPS base URL is `https://dustwave-media-staging.jogo.workers.dev`, and toggle the service active.
8. In Accounts, connect each Instagram account through the first-class Instagram account flow.

### Cloudflare R2 Media Staging

The app includes a Worker at `workers/media-staging` for short-lived media staging backed by Cloudflare R2. This is required for providers such as Instagram that need to fetch media from a public HTTPS URL.

Code and infrastructure status:

- Worker stage/serve/delete/cleanup endpoints are implemented, and an hourly scheduled cleanup trigger removes expired objects without operator action.
- Desktop Media Staging service entry is implemented.
- Local Instagram publishing stages static images through the Worker and records staged object metadata in the publish result.
- R2 bucket `dustwave-media-staging` is created.
- Worker `dustwave-media-staging` is deployed at `https://dustwave-media-staging.jogo.workers.dev`.
- `MEDIA_STAGING_TOKEN`, `CLOUDFLARE_API_TOKEN`, and `CLOUDFLARE_ACCOUNT_ID` are set in GitHub secrets.
- `MEDIA_STAGING_WORKER_NAME`, `MEDIA_STAGING_BUCKET_NAME`, `PUBLIC_MEDIA_BASE_URL`, `MEDIA_STAGING_MAX_OBJECT_BYTES`, and `MEDIA_STAGING_DEFAULT_TTL_SECONDS` are set in GitHub variables. Optional `MEDIA_STAGING_CLEANUP_CRON` can override the default hourly cleanup schedule.
- Live stage/serve/delete smoke test passed.

Remaining manual acceptance:

1. In Dust Wave Services on the launch Mac, save the Media Staging token from the secure local secret store or GitHub secret, set the HTTPS base URL, and toggle the service active. A non-interactive macOS Keychain write was rejected locally, so use the app form if the CLI cannot write the Keychain.
2. During acceptance, publish an Instagram image from local media and confirm the staged object URL expires, is deleted after publish, or is removed by the hourly scheduled cleanup.

### TikTok

TikTok is MVP scope for assisted publishing and analytics. Direct API publishing remains approval-gated until TikTok approves `video.upload` or `video.publish` for the app. Do not store the TikTok client secret in the desktop app.

1. Open TikTok for Developers from Services or go to `https://developers.tiktok.com/`.
2. Create or update the Dust Wave TikTok app.
3. Deploy or prepare the bundled broker in `workers/tiktok-broker`:
   - Current scaffold: the D1 database exists, the initial migration has been applied, and the Worker health endpoint is deployed at `https://dustwave-tiktok-broker.jogo.workers.dev/api/health`.
   - For a fresh environment only, create the D1 database with `npx wrangler d1 create dustwave-tiktok-broker`, then export the returned `database_id` as `TIKTOK_BROKER_D1_DATABASE_ID`.
   - Generate the ignored deploy config with `npm run tiktok:broker:config:check`.
   - Set Worker secrets with `npx wrangler secret put TIKTOK_CLIENT_KEY --config workers/tiktok-broker/wrangler.generated.jsonc` and `npx wrangler secret put TIKTOK_CLIENT_SECRET --config workers/tiktok-broker/wrangler.generated.jsonc`.
   - `TOKEN_ENCRYPTION_KEY` and `BROKER_ADMIN_TOKEN` have been generated into the local `Apple Auth` folder and set in GitHub/Cloudflare. Regenerate only if you intend to rotate broker credentials.
   - Apply migrations with `npm run tiktok:broker:migrate`.
   - Deploy with `npm run tiktok:broker:deploy`.
   - Optional GitHub Actions path: `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` are configured in GitHub already. If the broker workflow reports D1 permission errors, rotate the token with Workers and D1 permissions, keep the broker repository secrets/variables documented in `workers/tiktok-broker/README.md`, then run the `TikTok Broker` workflow with `deploy_broker=true` and `apply_migrations=true`.
4. Configure the OAuth callback URL to the broker callback shown in the Services packet, for example `https://<dust-wave-broker>/api/tiktok/oauth/callback`.
5. Request or confirm the MVP analytics scopes:
   - `user.info.basic`
   - `user.info.stats`
   - `video.list`
6. Request `video.upload` and/or `video.publish` only for the later Send to TikTok or Direct API publishing modes. Until approval is granted, keep TikTok publishing mode set to Assisted.
7. Store the TikTok client secret only as a Cloudflare Worker secret for the broker.
8. Copy the TikTok Client Key into Services.
9. Set the TikTok Broker URL to the deployed HTTPS broker base URL.
10. Set Publishing Mode to Assisted unless TikTok has approved a stronger mode.
11. Set the TikTok service Active.
12. Save Service.
13. Confirm the service card says configured and active.

### Unsplash

1. Open Unsplash applications from Services or go to `https://unsplash.com/oauth/applications`.
2. Create or update the Dust Wave Unsplash app.
3. Confirm public demo or production access is enough for the expected MVP search volume.
4. Copy the Unsplash access key into Services as API Key.
5. Save the credential field.
6. Set the Unsplash service Active.
7. Save Service.
8. Search Unsplash from the Media flow during acceptance and confirm downloaded files enter the media library correctly.

### Klipy

1. Open the Klipy Partner Panel from Services or go to `https://partner.klipy.com/`.
2. Create or update the Dust Wave Klipy app.
3. Start with the test key if needed, then request production access before MVP launch.
4. Configure Klipy content filters and blocklisted keywords for Dust Wave's use case.
5. Confirm the required attribution/branding guidance. The app treats Klipy GIFs as provider references, not permanent reusable media files.
6. Copy the Klipy key into Services as API Key.
7. Save the credential field.
8. Set the Klipy service Active.
9. Save Service.
10. During acceptance, verify GIF search and preview work, and verify Klipy GIFs are not saved into the reusable media library.

### Mastodon

Mastodon does not use the Services credential store for a single global provider app in the same way as X/Facebook/TikTok/Unsplash/Klipy. Onboarding happens per server in Accounts.

Before account onboarding:

1. Decide which Mastodon servers Dust Wave will use.
2. Confirm dynamic app registration is acceptable for those servers, or register a server-specific app through Accounts.
3. Keep server names in host-only form such as `mastodon.social`, not a full profile URL.

### Provider Setup Exit Criteria

Provider setup is complete when:

- X, Facebook, TikTok, Unsplash, and Klipy service cards show configured and active where those services are in MVP scope.
- Instagram is explicitly represented in account onboarding and acceptance, even though it uses the shared Meta service credentials.
- Copy Missing does not list any MVP service that still needs setup.
- A copied setup packet still contains no secret values.
- System logs and provider error messages do not show API keys, client secrets, access tokens, or refresh tokens.
- Klipy production terms/attribution/content-filter gate is recorded in `docs/GIF_PROVIDER_DECISION.md`.
- TikTok client secret is stored only in the broker secret store, and the desktop app stores only the client key, broker URL, and per-account opaque broker connection credentials.

## P0 2.5: Local AI Media Labs

Goal: ship local-only AI media tools as MVP scope without creating cloud dependency, hidden data movement, or irreversible media edits.

LiteRT.js is the primary MVP runtime. See `docs/LITERT_MVP_EVALUATION.md` for the runtime comparison, model-selection policy, and acceptance gates. ONNX Runtime Web, Transformers.js, or MediaPipe may be used only as local packaged adapters for a specific feature when LiteRT.js cannot satisfy model quality, licensing, or runtime acceptance.

Implementation status: the Labs flag, bundled LiteRT Wasm runtime, bundled Real-ESRGAN-x4plus `w8a8` TFLite model, runtime/model compile probe, tiled model-backed x4 upscaling derivatives, deterministic fallback upscaling, deterministic crop derivatives, media preflight, local profile-backed media search, review-required profile-backed alt-text drafts, derivative metadata with source/output hashes, and LiteRT/model asset release checks are implemented. Model-backed output quality still requires operator acceptance in the packaged signed app.

MVP feature set:

1. Local image upscaling.
2. Media quality preflight.
3. Smart crop suggestions.
4. Local semantic media search.
5. Alt-text drafting.

Implementation requirements:

- Gate the tools behind a Settings or Labs flag.
- Bundle LiteRT.js Wasm locally; do not load runtime assets from a CDN.
- Use only model weights with reviewed redistribution rights, recorded source URLs, bundled notices, file sizes, and SHA-256 checksums.
- Do not add cloud fallback or remote model calls.
- Preserve originals and save AI outputs as derivative media.
- Store derivative metadata: source media ID, model name/version/license, runtime, file hashes, operation type, and creation time.
- Provide progress, cancellation, retry, and failure states.
- Feature-detect WebGPU and provide Wasm fallback or clear disablement.
- Keep generated alt text as a draft that requires operator review.
- Validate in the signed/stapled packaged Tauri app with network disabled.

Exit criteria:

- The app can run a local AI capability probe from the packaged app.
- The probe can read the bundled model manifest and compile the packaged Real-ESRGAN-x4plus model with WebGPU/Wasm fallback.
- Upscaling creates a reviewed derivative without overwriting the original.
- Preflight warnings are deterministic, provider-aware, and non-destructive.
- Smart crop suggestions require operator acceptance before creating derivatives.
- Semantic media search works without network access.
- Alt-text drafting is visibly marked as generated and remains editable before publishing.
- Backup/restore preserves local AI derivatives and metadata.
- Support export redacts embeddings and generated metadata unless explicitly included.

## P0 3: Dust Wave Account Onboarding

Goal: every Dust Wave account that should be managed in the MVP is inventoried, connected, authorized, refreshed, and import-ready where supported.

### Build The Account Inventory

1. Open Dust Wave Social.
2. Go to Accounts.
3. Click Copy Intake CSV.
4. Paste the CSV into a private spreadsheet or working doc.
5. Fill one row for every account Dust Wave expects to manage.

Use these columns exactly:

- `provider`: `twitter`, `facebook_page`, `instagram`, `mastodon`, or `tiktok` for MVP-supported connected accounts. Put unsupported providers in `notes` for future/manual workflow coverage.
- `display_name`: human-readable account/page name.
- `handle_or_page_id`: handle, username, page ID, or server/profile identifier.
- `owner`: internal Dust Wave owner responsible for access and content approval.
- `posting_allowed`: `yes` only when the app is allowed to publish to that account.
- `import_history`: `yes` when historical posts/metrics should be imported.
- `notes`: permissions, special restrictions, approval notes, provider gaps, or manual workflow notes.

### Connect X/Twitter Accounts

Prerequisite: X service is configured and active in Services.

1. Go to Accounts.
2. Click Add Account.
3. In X / Twitter, confirm the redirect URI is `http://localhost/callback`.
4. Click Start X.
5. Click Authorize.
6. Complete the provider authorization in the browser.
7. If the browser lands on a localhost callback page, copy the `code` query parameter from the URL.
8. Paste the code into Authorization code.
9. Click Connect.
10. Confirm the connected account appears as authorized.
11. Click Refresh on the account card.
12. Click Import or Queue if `import_history` is `yes`.

### Connect Facebook Page Accounts

Prerequisite: Facebook service is configured and active in Services, and the Meta app has access to the target Pages.

1. Go to Accounts.
2. Click Add Account.
3. In Facebook Page, confirm the redirect URI is `http://localhost/callback`.
4. Click Start Facebook.
5. Click Authorize.
6. Complete the provider authorization in the browser.
7. If the browser lands on a localhost callback page, copy the `code` query parameter from the URL.
8. Paste the code into Authorization code.
9. Click List Pages.
10. Select the Pages Dust Wave should manage.
11. Click Save Pages.
12. Confirm each Page account appears as authorized.
13. Click Refresh on each Page account card.
14. Click Import or Queue if `import_history` is `yes`.

### Connect Instagram Accounts

Prerequisite: the first-class Instagram provider implementation is complete, the shared Meta service is configured and active in Services, the Meta app has Instagram permissions, and the target Instagram accounts are professional accounts connected to the required Meta business/Page assets.

1. Go to Accounts.
2. Click Add Account.
3. In Instagram, confirm the redirect URI is `http://localhost/callback`.
4. Start the Instagram/Meta authorization flow.
5. Complete the provider authorization in the browser.
6. If the browser lands on a localhost callback page, copy the `code` query parameter from the URL.
7. Paste the code into Authorization code.
8. List available Instagram accounts.
9. Select the Instagram accounts Dust Wave should manage.
10. Save accounts.
11. Confirm each Instagram account appears as authorized.
12. Click Refresh on each Instagram account card.
13. Click Import or Queue if `import_history` is `yes`.
14. Confirm the report shows supported Instagram media and insight metrics after import.

### Connect Mastodon Accounts

Prerequisite: Dust Wave knows the Mastodon server host for the account.

1. Go to Accounts.
2. Click Add Account.
3. In Mastodon, enter the server host, client name, and website.
4. Click Register.
5. Click Authorize.
6. Complete the provider authorization in the browser.
7. Copy the returned authorization code.
8. In the Mastodon connect form, enter the same server host and the authorization code.
9. Click Connect.
10. Confirm the account appears as authorized.
11. Click Refresh on the account card.
12. Click Import or Queue if `import_history` is `yes`.

### Connect TikTok Accounts

Prerequisite: TikTok service is configured and active in Services, the broker is deployed, and the broker has completed TikTok OAuth for the account with `user.info.basic`, `user.info.stats`, and `video.list`.

1. Go to Accounts.
2. Open the broker start URL in a browser: `https://<dust-wave-broker>/api/tiktok/oauth/start`.
3. Authorize the TikTok account.
4. On the broker completion page, copy the TikTok user ID, display name, username, granted scopes, and one-time broker connection credential.
5. Click Add Account.
6. In TikTok, enter the TikTok user ID, display name, username, and granted scopes.
7. Paste the opaque broker connection credential issued by the broker.
8. Click Connect.
9. Confirm the account appears as authorized.
10. Click Import or Queue if `import_history` is `yes`.
11. Confirm the report shows TikTok views, likes, comments, shares, and audience data after import.
12. Use assisted publishing for TikTok until `video.upload` or `video.publish` is approved and a stronger publishing mode is explicitly enabled.

### Account Onboarding Exit Criteria

Account onboarding is complete when:

- Every MVP account is present in the inventory sheet.
- Every supported account is connected through Accounts.
- Connected account cards show authorized.
- Refresh succeeds for each refresh-capable connected account.
- Import or Queue has been run for each import-ready account.
- Instagram publishing, scheduling, and insights imports pass live acceptance for every Instagram account in MVP scope.
- TikTok analytics imports succeed through the broker for every TikTok account in MVP scope.
- Copy Plan includes current service readiness, connected-account counts, blank intake CSV, and connected-account inventory CSV.
- Unsupported providers are captured in notes as manual workflows or future provider work.
- OS keychain secrets are not included in backups or onboarding exports.

## P1 Acceptance After P0

Run these after P0 2 and P0 3 are complete:

1. Publish text-only, image, video, and GIF/media test posts where direct publishing is supported.
2. For Instagram, publish and schedule representative supported media types for Dust Wave's real accounts, then verify provider URLs or remote IDs, insights import, and provider-side error handling.
3. For TikTok, complete one assisted publishing test and one analytics import/report test.
4. Schedule a future post for each direct-publishing provider and verify the local worker publishes it and stores the provider URL or remote ID.
5. Force or simulate rate limits and verify retry/defer behavior.
6. Revoke one provider token or broker connection and verify health checks, failed jobs, and reconnect flows.
7. Verify character limits, media counts, media type restrictions, direct-publishing approval gates, and missing-account validation.
8. Verify X/Twitter refuses simultaneous posting to multiple X accounts.
9. Test backup/restore on a clean app-data directory.
10. Publish a test update from `0.1.0` to a higher version.

## P2 Launch Readiness

1. Run the product risk review in `docs/BEST_PRACTICES.md`.
2. Red-team publish-now, schedule, duplicate, retry, bulk delete, account connection, backup/restore, media import, and support export.
3. Complete final visual QA against Pool and Store.
4. Confirm Gambado font redistribution rights before broader public distribution.
5. Regenerate and review the `Current Release Candidate` section in this document with `npm run mvp:release:notes`.
6. Tag the release and publish only notarized/stapled artifacts.
7. Define owners for provider credentials, Apple credentials, update hosting, backups, support, and release approval.
8. Decide when the legacy Laravel/Mixpost system can be archived after live-provider acceptance.

## Consolidated Release Engineering Reference

This section replaces the old separate release checklist and finish-line checklist. Keep launch operations here so the launch state has one source of truth.

### Local Preflight

Run these before cutting or publishing a desktop build:

```sh
npm ci
npm run desktop:release:preflight
npm run mvp:launch:readiness
npm run mvp:release:notes
npm run mvp:release:notes:check
npm run local-ai:models:check
npm run desktop:release:check
```

`desktop:release:preflight` reports local release inputs such as sidecars, updater keys, signing variables, provider credential environment variables, and CLI availability without printing secrets.

`desktop:release:check` builds the desktop Vue bundle, verifies Rust formatting, runs Rust tests, desktop UI contract tests, TikTok broker tests, and media staging Worker tests.

`mvp:release:notes` updates the generated release-candidate section in this document from the current signed artifact paths, checksums, updater metadata, notarization submission, readiness summary, and rollback plan.

### CI Preflight

`.github/workflows/desktop.yml` runs `npm run desktop:release:check` on pushes to `main` or `master` and on pull requests.

The Desktop workflow has a manual `workflow_dispatch` path. Use `build_bundle=true` to build and upload macOS `.app.zip` and `.dmg` artifacts. Optional inputs request media sidecars and updater artifacts. Use `publish_release=true` with a `release_tag` only after the notarized DMG, release notes, and rollback plan are ready.

`.github/workflows/tiktok-broker.yml` runs TikTok broker boundary tests and can deploy the broker plus apply D1 migrations when the required Cloudflare and TikTok secrets are configured.

`.github/workflows/media-staging.yml` runs Cloudflare R2 media staging Worker tests, Wrangler dry-run checks when variables are configured, and can deploy the Worker.

GitHub Actions bundle builds are signed but do not notarize by default. Public distribution still requires a notarized, stapled, Gatekeeper-accepted DMG.

### Apple Auth Folder

Local signing automation reads release credentials from the sibling iCloud folder `Apple Auth`. Do not commit these files.

- `developer-id-application.p12`: Developer ID Application certificate.
- `apple-p12-password.txt`: password for the `.p12`.
- `AuthKey_<key-id>.p8`: App Store Connect API key file for notarization.
- `apple-api-issuer.txt`: App Store Connect issuer UUID.
- `tauri-updater-private.key`, `tauri-updater-password.txt`, and `tauri-updater-public-key.txt`: Tauri updater signing material.

Generate updater signing material once with:

```sh
npm run desktop:updater:keys
```

The updater private key and password stay outside the repository. Losing the updater private key means installed apps cannot accept future updates signed by a replacement key.

### Build Commands

Unsigned local verification build:

```sh
npm run desktop:build
```

Signed Apple Silicon release build with bundled media sidecars:

```sh
npm run desktop:release:build:signed:with-media
```

Signed Apple Silicon release build with media sidecars and updater artifacts:

```sh
npm run desktop:release:build:signed:with-media-and-updater
```

After local verification, remove reproducible desktop build output without touching `node_modules`, source files, docs, or staged FFmpeg/FFprobe sidecars:

```sh
npm run desktop:clean
```

Do not clean build output while notarization is pending or while the current launch candidate still needs local artifact checks.

### macOS Signing And Notarization

For public macOS distribution outside the App Store, use a Developer ID Application certificate.

Common release variables:

- `APPLE_CERTIFICATE`: base64 encoded `.p12` signing certificate.
- `APPLE_CERTIFICATE_PASSWORD`: password used when exporting the `.p12`.
- `APPLE_SIGNING_IDENTITY`: optional explicit signing identity.
- `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`: Apple ID notarization path.
- `APPLE_API_KEY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_PATH`: App Store Connect API key notarization path.
- `APPLE_PROVIDER_SHORT_NAME`: only when the Apple ID maps to multiple teams.

Before notarization returns, verify signed artifacts without requiring a stapled ticket:

```sh
npm run desktop:release:artifact-check -- --require-updater
```

Submit and staple:

```sh
npm run desktop:macos:notarize
```

If Apple accepts the upload but local waiting times out, keep the submitted DMG in place and resume:

```sh
npm run desktop:macos:notarize:wait -- <submission-id>
```

After stapling:

```sh
npm run desktop:release:artifact-check -- --require-updater --require-stapled
```

### Updater Rules

Use GitHub Actions and GitHub Releases for update artifacts.

The final release repository slug is `aindaco1/social`, and local `origin` points there.

`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `TAURI_UPDATER_PUBLIC_KEY`, and `DUSTWAVE_RELEASE_REPO` are configured in GitHub secrets/variables.

The generated updater overlay lives at `src-tauri/tauri.updater.generated.conf.json` and is ignored by git. Keep `src-tauri/tauri.updater.example.json` as the documented shape.

Updater builds generate `src-tauri/target/release/bundle/latest.json` beside the signed macOS artifacts. Upload `latest.json`, `.app.tar.gz`, and `.app.tar.gz.sig` beside the DMG in GitHub Releases.

### FFmpeg And FFprobe Sidecars

MVP decision: Apple Silicon macOS only. Intel and universal macOS sidecars are out of scope unless Dust Wave changes distribution targets.

Apple Silicon LGPL-only FFmpeg/FFprobe sidecars are staged from official FFmpeg source with:

```sh
npm run desktop:media:build-lgpl
```

Do not ship arbitrary Homebrew binaries as release sidecars. Dust Wave policy is LGPL-only FFmpeg/FFprobe sidecars. The staging script rejects binaries that report GPL licensing, `--enable-gpl`, `--enable-nonfree`, or common GPL codec flags.

For every release with media sidecars, keep the FFmpeg/FFprobe versions, configure lines, source archive/commit, license output, and binary source recorded in `THIRD_PARTY_NOTICES.md`.

### Desktop Permissions

The default Tauri capability intentionally grants only the plugin commands used by the desktop UI:

- `dialog:allow-open` for local media file selection.
- `notification:default` for native desktop health and publishing notifications.
- `opener:allow-open-url` plus `opener:allow-default-urls` for OAuth browser handoff.
- `updater:default` for signed update check, download, and install from System.

Do not broaden these to plugin defaults unless a new UI workflow needs the extra command surface and this launch plan documents the reason.

## Design, QA, Hygiene, And Operations

### Design And Brand

- MVP decision: ship with the Gambado typography treatment from Pool/Store. Confirm redistribution rights before broader public distribution.
- The Gambado OpenType files are bundled in `resources/fonts` and registered in `resources/css/fonts.css`.
- The desktop app uses the `dust-wave-square` logo from Pool assets for the sidebar mark, Tauri PNG icons, and macOS `.icns`.
- Review the desktop UI on a real Mac at 1024px, 1280px, and wide desktop widths, then compare against Pool/Store for typography, control density, panel rhythm, and button treatment.
- Keep production labels focused on social account management, publishing, media, reports, settings, and system logs. Do not reintroduce migration milestones, release readiness, schema, raw database, or raw queue inspection panels to the shipped app.

### QA And Acceptance

- Run `npm ci` from a clean checkout.
- Run `npm run desktop:release:check`.
- Run `npm run media:staging:wrangler:check` after media staging Worker or Cloudflare variable changes.
- Run `npm run desktop:release:artifact-check -- --require-updater --require-stapled`.
- Run `npm run desktop:smoke:launch`.
- Test first launch with no existing app-data directory.
- Test relaunch with existing app-data and pending jobs.
- Test settings save/load, default accounts, timezone, date format, week start, and desktop notification preference.
- Test system logs refresh, export, and clear.
- Test Copy App Data Path from System.
- Test bulk delete, duplicate post, validate post, retry failed post, recover stale jobs, and queue account imports.
- Do final visual QA against Pool and Store.

### Product Risk Gate

Run the product risk review in `docs/BEST_PRACTICES.md` before public release and before later releases that change publishing, automation, account connection, media import, backup/restore, reporting, notifications, support exports, or provider behavior.

Confirm the release does not materially increase misinformation, impersonation, spam, harassment, doxxing, coordinated abuse, accidental posting from the wrong account, or account takeover risk without mitigation.

Every red flag must have an owner, mitigation, and ship/no-ship decision before release.

### Repository Hygiene

- Review untracked files before staging.
- Do not commit `src-tauri/target`, `resources/desktop/dist`, notarized DMGs, generated signing overlays, generated updater overlays, Worker `.dev.vars`, or local app-data.
- Add CI secrets for signing, notarization, updater signing, and provider smoke-test credentials only in secure secret stores.
- Tag the first signed release with a clear version, release notes, and migration warnings.

### Operations

- Define who owns provider credentials, Apple credentials, update hosting, backups, support, and release approvals.
- Define backup frequency and retention for local app data.
- Define support policy for corrupted local databases, failed migrations, revoked tokens, missed scheduled posts, and bad releases.
- Decide when legacy Laravel/Mixpost can be archived after parity, migration verification, and live-provider acceptance are complete.
- Keep a final launch-day checklist: signed build installed, accounts connected, test publish passed, backup exported, updater tested, rollback installer available.

## Official References

- X OAuth 2.0: `https://docs.x.com/fundamentals/authentication/oauth-2-0/authorization-code`
- Meta Instagram Content Publishing: `https://developers.facebook.com/docs/instagram-platform/content-publishing`
- TikTok Login Kit: `https://developers.tiktok.com/doc/login-kit-web/`
- TikTok scopes: `https://developers.tiktok.com/doc/scopes-overview`
- Unsplash API: `https://unsplash.com/documentation`
- Klipy Developers: `https://klipy.com/developers`
- Tauri Updater: `https://v2.tauri.app/plugin/updater/`
