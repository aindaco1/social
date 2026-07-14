# Security Policy

Dust Wave Social handles social-provider credentials, local account data, media, scheduled publishing jobs, reports, backups, and desktop support exports. Security reports should cover both traditional software vulnerabilities and product-abuse paths that could expose secrets, publish without clear operator intent, or help compromise social accounts.

## Reporting a Vulnerability

Do not open a public issue for a vulnerability or active abuse path. Report it privately to the Dust Wave maintainer responsible for this repository. If that contact is not available, use the original Mixpost security contact until the repository metadata is updated: dima@inovector.com.

Include:

- A concise description of the issue and affected workflow.
- Steps to reproduce, proof-of-concept details, logs, screenshots, or sample files when safe to share.
- Whether access tokens, refresh tokens, client secrets, API keys, local database files, backups, media, or account identifiers are exposed.
- Whether the issue can publish, schedule, retry, duplicate, delete, import, export, or restore data without clear operator intent.
- Whether the issue could support impersonation, harassment, spam, doxxing, misinformation, account takeover, or other abuse.

Do not include live secrets in the report. Redact tokens and keys unless the maintainer explicitly asks for a secure transfer path.

## In Scope

- Token, API key, client secret, or refresh-token exposure.
- Keychain/SQLite separation failures that store raw secrets in local data.
- Backup, restore, setup-packet, onboarding-packet, log, or support-export leaks.
- OAuth callback, account connection, account refresh, or provider authorization bypasses.
- Publishing, scheduling, retry, duplicate, or bulk actions that can run without clear operator intent.
- Media import, URL download, thumbnail generation, or cleanup issues that expose or delete files unexpectedly.
- Updater, signing, notarization, or release-artifact integrity issues.
- Abuse paths that materially enable impersonation, harassment, spam, doxxing, misinformation, or account takeover.

## Product Risk Reports

Some issues are not classic security bugs but still block a responsible release. Use `docs/BEST_PRACTICES.md` as the red-flag standard. Report product risks privately when public disclosure would help abuse the app or compromise accounts; otherwise record them in the issue tracker with an owner, mitigation, and ship/no-ship decision.
