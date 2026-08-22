# Dust Wave Social Support Runbook

Updated: 2026-08-22

This runbook covers support actions that should be available without adding a raw database or developer panel to the production app.

## Find Local App Data

1. Open Dust Wave Social.
2. Go to System.
3. Use Copy App Data Path.
4. Paste the path into Finder with Go To Folder, or send the copied path to support.

Do not ask operators to browse the SQLite database directly unless engineering is handling a private recovery session.

## Repeated SecurityAgent prompts in local development

Production credentials use the `com.dustwave.social` macOS Keychain service. A changing ad-hoc debug signature produces a new code requirement after each rebuild, so macOS can ask for access repeatedly even when the credential itself is unchanged.

1. Start local development with `npm run desktop:dev`; its Cargo runner signs the debug executable with an installed Developer ID identity and the stable `com.dustwave.social` identifier before launch.
2. Confirm `codesign -dr - src-tauri/target/debug/dust-wave-social` reports an identifier-based Apple Developer ID requirement, not a `cdhash` requirement.
3. If no Developer ID identity is installed, keep the debug build in its default environment-only credential mode. Do not choose Always Allow for an ad-hoc requester or weaken the saved Keychain item's access controls.
4. Use the signed production app for provider acceptance. An environment-only debug build intentionally cannot save account or provider secrets to Keychain.

## First Response For Failed Publishing

1. Open System and use Copy Info.
2. Open Posts and filter to Failed.
3. Open the failed post detail and record the provider, account, error message, and scheduled time.
4. Refresh the affected account connection from Connections > Connected accounts.
5. Retry only after the provider connection is authorized and service credentials are active.

If multiple posts failed at once, check System for provider limits or failed background work before retrying each post.

## TikTok Broker Issues

TikTok MVP support uses assisted publishing plus broker-backed analytics imports. The desktop app stores only the broker URL, TikTok client key, and each account's opaque broker connection credential.

When TikTok import fails:

1. In Connections > Provider setup, confirm TikTok is Active, has a saved Client Key, and has an HTTPS Broker URL.
2. In Connections > Connected accounts, confirm the TikTok account is authorized and the granted scopes include `user.info.basic`, `user.info.stats`, and `video.list`.
3. Open `https://<broker>/api/health` and confirm the broker responds.
4. Re-run Import on the TikTok account.
5. If the broker credential was revoked or lost, open `https://<broker>/api/tiktok/oauth/start`, authorize again, and paste the new broker connection credential into a reconnected TikTok account.

To inspect non-secret broker status for one account, call the status endpoint with the broker admin token:

```sh
curl \
  -H "Authorization: Bearer $BROKER_ADMIN_TOKEN" \
  "https://<broker>/api/tiktok/accounts/<tiktok-open-id>/status"
```

To revoke all desktop broker credentials for a TikTok account during an incident, call the broker revocation endpoint with the broker admin token:

```sh
curl -X POST \
  -H "Authorization: Bearer $BROKER_ADMIN_TOKEN" \
  "https://<broker>/api/tiktok/accounts/<tiktok-open-id>/revoke-connections"
```

Then revoke the TikTok app authorization in TikTok if account access may be compromised.

## Instagram Local Media Issues

Instagram publishing of local desktop images uses the Cloudflare R2 Media Staging Worker at `https://dustwave-media-staging.jogo.workers.dev`. The Worker creates short-lived public HTTPS object URLs because Meta must fetch the image during publishing. Expired objects are cleaned up through both the authenticated cleanup endpoint and the hourly scheduled Worker trigger.

For an ordinary app user, create a one-time pairing code on a provisioned operator Mac with `npm run media:staging:enrollment -- --label "Name of Mac"`. Send the code privately. The user opens Connections > Provider setup > Instagram Local Media, pastes the code under Pair this Mac, and selects Pair This Mac. They do not need a Cloudflare account, Wrangler, an R2 bucket name, or the reusable Worker token.

When Instagram publishing fails before the provider publish call:

1. In Connections > Provider setup, confirm Instagram Local Media is Active, is paired, and uses `https://dustwave-media-staging.jogo.workers.dev` as the Service URL.
2. Open `https://dustwave-media-staging.jogo.workers.dev/api/health` and confirm the Worker responds.
3. Confirm the selected media is a static image. MVP Instagram direct publishing does not support GIFs, videos, reels, stories, or carousels yet.
4. Re-run the publish attempt.
5. If a setup code fails, issue a new code; codes expire after 15 minutes and cannot be reused.
6. If staging still fails for every paired device, use `npm run media:staging:operator:plan` before changing secrets. Do not replace `MEDIA_STAGING_TOKEN`; the additive provisioning command preserves current installations and refuses to overwrite an existing `MEDIA_STAGING_TOKEN_NEXT`.
7. If old staged objects accumulate, confirm the Worker was deployed with `triggers.crons` active or call the authenticated `/api/media/cleanup` endpoint with an operator token.

Do not make the R2 bucket public. Public access should stay behind the Worker so object names remain high-entropy, temporary, and cleanup-aware.

## Local AI Media Labs Issues

Local AI Media Labs runs only inside the desktop app with bundled runtime assets and bundled model weights. It must not download models at runtime or fall back to cloud inference.

When Local AI probe or Upscale fails:

1. In Settings, confirm Local AI media is enabled.
2. In Media, use Probe LiteRT and record whether the panel reports WebGPU, Wasm fallback, or an error.
3. Confirm the selected media is a static local image, not a GIF, video, external provider reference, or missing file.
4. Re-run Upscale with the app online and then with Wi-Fi disabled if this is release acceptance.
5. If the operator cancels, confirm no partial derivative appears in Media.
6. If a derivative is created, review it before publishing. The original media should remain available, and the derivative metadata should include the source media, model/runtime details, dimensions, and SHA-256 hashes.
7. If generated output is poor, delete the derivative and keep the original. Treat model quality as an acceptance issue, not a provider failure.

Backups include Local AI derivative media and metadata, but OS keychain secrets remain excluded. Exported system logs should be used for support before asking for a full backup.

## Stop Scheduled Publishing In An Emergency

1. Quit Dust Wave Social to stop the local worker loop.
2. Disconnect or revoke the compromised provider credential in the provider portal if account access may be compromised.
3. Reopen Dust Wave Social only after the provider account is safe.
4. In Posts, move any risky scheduled posts back to draft or delete them.
5. In System, export logs before clearing anything.

For account compromise, preserve logs and do not reconnect the account until the provider password, app permissions, and recovery email are verified.

## Backup And Restore

1. Create Backup from System before risky troubleshooting.
2. Confirm the backup manifest path appears in the app.
3. Restore only from a backup folder created by Dust Wave Social.
4. After restore, reconnect provider accounts because OS keychain secrets are intentionally not included.
5. Run a test draft save, media preview, and account refresh before resuming scheduled publishing.

Backups include the local database, app-owned media, and manifest. They do not include access tokens, refresh tokens, client secrets, API keys, or OS keychain material.

## Support Export Hygiene

- Do not request screenshots of provider portals that include client secrets, app secrets, access tokens, or webhook signing secrets.
- Prefer Copy Info and exported system logs over raw database files.
- Redact social account private metadata unless it is necessary for the case.
- Treat accidental public posting, impersonation, account takeover, harassment, spam, doxxing, and misinformation reports as release-risk incidents, not ordinary UI bugs.

## Escalation

Escalate to engineering when:

- A backup will not restore.
- A scheduled post publishes after it was moved to draft or deleted.
- Logs expose tokens or secrets.
- The updater downloads but fails signature validation.
- The app opens on one Mac but fails Gatekeeper or crashes on a clean target Mac.
- Provider APIs reject a post that passed local validation.
