# Security Policy

Dust Wave Social handles social-provider credentials, local account data, media, scheduled publishing jobs, reports, backups, and desktop support exports. Security reports should cover both traditional software vulnerabilities and product-abuse paths that could expose secrets, publish without clear operator intent, or help compromise social accounts.

## Reporting a Vulnerability

Do not open a public issue for a vulnerability or active abuse path. Report it privately to the Dust Wave maintainer responsible for this repository, or use GitHub private vulnerability reporting for `aindaco1/social` when that channel is enabled.

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

## Automatic Update Check

Dust Wave Social makes one background request to its public signed release feed when the app opens. The request is limited to update discovery and ordinary network metadata; it does not include social credentials, connected-account data, posts, media, reports, logs, or device profiling. The app does not automatically download or install releases. Installing a signed update and restarting the app requires an explicit operator action.

## Product Risk Reports

Some issues are not classic security bugs but still block a responsible release. Use `docs/BEST_PRACTICES.md` as the red-flag standard. Report product risks privately when public disclosure would help abuse the app or compromise accounts; otherwise record them in the issue tracker with an owner, mitigation, and ship/no-ship decision.

## 0.1.10 dependency review

The 2026-09-06 production npm audit identified the existing Tiptap 2.x dependency
through [GHSA-cp6q-959q-f8rh](https://github.com/ueberdosis/tiptap/security/advisories/GHSA-cp6q-959q-f8rh).
The upstream fix is in Tiptap 3.30.4; this release does not claim the 2.x dependency
itself is patched or that the production npm audit is clean.

The reviewed desktop composer passes string content into a fixed Document/Div/Text/Link
schema, does not accept imported attribute objects or dynamic HTMLAttributes, and
uses a CSP without inline JavaScript permission. A regression test using the actual
schema and JSON-origin prototype/event attributes confirms those fields do not reach
the rendered attribute object. Another contract protects the current string/static
attribute boundary. This is a scoped exposure assessment, not a blanket guarantee:
adding custom attributes, imported editor JSON, or dynamic extensions requires a new
review. Upgrading the editor to the patched major version remains maintenance work.

The transitive `qs` dependency was updated to the patched 6.16.0 release for
[its parsing advisories](https://github.com/ljharb/qs/security/advisories/GHSA-x5fp-wj9c-mxmx).
It is used by the retained Inertia web package, not by the desktop entry point.
Development-server findings are separate from packaged application exposure.
