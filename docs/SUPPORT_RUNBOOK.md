# Dust Wave Social Support Runbook

Updated: 2026-07-13

This runbook covers support actions that should be available without adding a raw database or developer panel to the production app.

## Find Local App Data

1. Open Dust Wave Social.
2. Go to System.
3. Use Copy App Data Path.
4. Paste the path into Finder with Go To Folder, or send the copied path to support.

Do not ask operators to browse the SQLite database directly unless engineering is handling a private recovery session.

## First Response For Failed Publishing

1. Open System and use Copy Info.
2. Open Posts and filter to Failed.
3. Open the failed post detail and record the provider, account, error message, and scheduled time.
4. Refresh the affected account connection from Accounts.
5. Retry only after the provider connection is authorized and service credentials are active.

If multiple posts failed at once, check System for provider limits or failed background work before retrying each post.

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
