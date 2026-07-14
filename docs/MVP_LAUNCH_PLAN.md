# Dust Wave Social MVP Launch Plan

Updated: 2026-07-14

Audience: Dust Wave operators preparing the Apple Silicon macOS MVP release.

This plan memorializes the remaining launch work after the Tauri/Rust desktop migration, Pool/Store design pass, Mixpost parity work, bundled LGPL-only FFmpeg/FFprobe sidecars, signing wrapper, updater scaffolding, and CI hardening.

## MVP Launch Definition

The MVP is ready to launch when Dust Wave can install a notarized Apple Silicon macOS app, connect production social/media provider credentials, onboard Dust Wave social accounts, publish and schedule representative posts, restore from backup, and recover from provider/auth failures without using development-only panels or raw database access.

## Current Status

- Green: desktop release checks, Rust tests, CI Desktop workflow, legacy PHP test workflow, asset workflow, signed local artifacts, bundled media sidecars, updater artifact generation, and pre-staple artifact verification.
- Yellow: Apple notarization submission is pending Apple acceptance. Keep the submitted DMG until the same artifact is stapled.
- Yellow: production provider credentials and live account acceptance are not complete.
- Yellow: updater publish/install test still needs a higher-version draft release.
- Red until completed: clean-Mac Gatekeeper install, live provider publishing validation, and final launch go/no-go.

## Critical Path

1. Finish Apple notarization and strict artifact verification.
2. Configure provider credentials in Services.
3. Onboard Dust Wave accounts in Accounts.
4. Validate live publishing, scheduling, media, imports, reports, and provider failure handling.
5. Publish a test update and define rollback.
6. Run ethical/security acceptance and final visual QA.
7. Create release notes, tag the release, publish notarized artifacts, and keep rollback artifacts available.

## P0 1: Apple Notarization

Status: waiting on Apple.

When Apple accepts submission `89124c3c-7510-41d2-a037-e3cbc1e8908c`, run:

```sh
npm run desktop:macos:notarize:wait -- 89124c3c-7510-41d2-a037-e3cbc1e8908c
npm run desktop:release:artifact-check -- --require-updater --require-stapled
```

Then install the stapled DMG on a clean Apple Silicon Mac and confirm Gatekeeper opens it without warnings.

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

### Facebook/Meta

1. Open Meta Developers from Services or go to `https://developers.facebook.com/apps`.
2. Create or update the Dust Wave app using the Pages use case.
3. Configure the OAuth callback URL as `http://localhost/callback`.
4. Configure the default Graph API version. Dust Wave defaults to `v25.0`.
5. Request or confirm the permissions needed for MVP Page publishing and reporting:
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
6. Complete Meta app review or role-based tester/admin setup as needed for Dust Wave's managed Pages.
7. Copy the Meta app values into Services:
   - App ID
   - App Secret
8. Save each credential field.
9. Set the Facebook service Active.
10. Save Service.
11. Confirm the service card says configured and active.

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

Mastodon does not use the Services credential store for a single global provider app in the same way as X/Facebook/Unsplash/Klipy. Onboarding happens per server in Accounts.

Before account onboarding:

1. Decide which Mastodon servers Dust Wave will use.
2. Confirm dynamic app registration is acceptable for those servers, or register a server-specific app through Accounts.
3. Keep server names in host-only form such as `mastodon.social`, not a full profile URL.

### Provider Setup Exit Criteria

Provider setup is complete when:

- X, Facebook, Unsplash, and Klipy service cards show configured and active where those services are in MVP scope.
- Copy Missing does not list any MVP service that still needs setup.
- A copied setup packet still contains no secret values.
- System logs and provider error messages do not show API keys, client secrets, access tokens, or refresh tokens.
- Klipy production terms/attribution/content-filter gate is recorded in `docs/GIF_PROVIDER_DECISION.md`.

## P0 3: Dust Wave Account Onboarding

Goal: every Dust Wave account that should be managed in the MVP is inventoried, connected, authorized, refreshed, and import-ready where supported.

### Build The Account Inventory

1. Open Dust Wave Social.
2. Go to Accounts.
3. Click Copy Intake CSV.
4. Paste the CSV into a private spreadsheet or working doc.
5. Fill one row for every account Dust Wave expects to manage.

Use these columns exactly:

- `provider`: `twitter`, `facebook_page`, or `mastodon` for MVP-supported connected accounts. Put unsupported providers in `notes` for future/manual workflow coverage.
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

### Account Onboarding Exit Criteria

Account onboarding is complete when:

- Every MVP account is present in the inventory sheet.
- Every supported account is connected through Accounts.
- Connected account cards show authorized.
- Refresh succeeds for each connected account.
- Import or Queue has been run for each import-ready account.
- Copy Plan includes current service readiness, connected-account counts, blank intake CSV, and connected-account inventory CSV.
- Unsupported providers are captured in notes as manual workflows or future provider work.
- OS keychain secrets are not included in backups or onboarding exports.

## P1 Acceptance After P0

Run these after P0 2 and P0 3 are complete:

1. Publish text-only, image, video, and GIF/media test posts where supported.
2. Schedule a future post and verify the local worker publishes it and stores the provider URL or remote ID.
3. Force or simulate rate limits and verify retry/defer behavior.
4. Revoke one provider token and verify health checks, failed jobs, and reconnect flows.
5. Verify character limits, media counts, media type restrictions, and missing-account validation.
6. Verify X/Twitter refuses simultaneous posting to multiple X accounts.
7. Test backup/restore on a clean app-data directory.
8. Publish a test update from `0.1.0` to a higher version.

## P2 Launch Readiness

1. Run the product risk review in `docs/BEST_PRACTICES.md`.
2. Red-team publish-now, schedule, duplicate, retry, bulk delete, account connection, backup/restore, media import, and support export.
3. Complete final visual QA against Pool and Store.
4. Confirm Gambado font redistribution rights before broader public distribution.
5. Draft release notes and rollback instructions.
6. Tag the release and publish only notarized/stapled artifacts.
7. Define owners for provider credentials, Apple credentials, update hosting, backups, support, and release approval.
8. Decide when the legacy Laravel/Mixpost system can be archived after live-provider acceptance.
