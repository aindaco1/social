# Dust Wave Social Best Practices

Audience: Dust Wave product, engineering, release, and social-operations owners.

This document defines product and release practices for Dust Wave Social beyond mechanical Mixpost feature parity. It is informed by the Ethical OS Toolkit from the Institute for the Future and Omidyar Network, copyright 2018, licensed CC BY-NC-SA 4.0, available at https://ethicalos.org/. The guidance below is paraphrased for Dust Wave Social's local desktop, social publishing, OAuth, media, reporting, and backup workflows.

Licensing note: keep the attribution above, do not copy long toolkit passages into this repository, and confirm CC BY-NC-SA compatibility before redistributing this guidance in a commercial package or public documentation site.

## Operating Principle

Dust Wave Social should help operators manage accounts safely, clearly, and reversibly. The app should not optimize for hidden growth loops, surprise data collection, avoidable credential risk, dark patterns, or publishing flows that make it easier to mislead, harass, impersonate, or coordinate abuse.

Before shipping a new workflow, ask about first-order, second-order, and third-order impacts:

- Who can be helped by this feature?
- Who can be harmed by this feature?
- How could a legitimate operator misuse it under stress, time pressure, or bad information?
- How could a malicious actor use it if they gained account access?
- What would surprise a user if it appeared in tomorrow's news?
- What safeguards, logs, confirmations, limits, or recovery paths would reduce the risk?

## Product Risk Review

Run this review before shipping new publishing, automation, account, analytics, media, backup, or provider workflows. Re-run it when provider APIs, business practices, or support policies materially change.

### Truth, Disinformation, And Propaganda

- Make the publishing state unambiguous: draft, scheduled, processing, published, failed, and retried posts must be clearly distinguishable.
- Preserve enough local history for operators to understand what was published, when it was published, which accounts were used, and which provider IDs were returned.
- Avoid features that make impersonation, fake attribution, or coordinated misinformation easier without explicit human review.
- Treat generated, edited, or externally sourced media as higher-risk content. Do not strip source metadata in ways that make operator review harder unless there is a clear privacy reason.
- Before enabling third-party media search in production, confirm the provider terms permit the app's actual workflow: previewing, selection, attribution, local thumbnail caching, permanent local storage if any, transient publish-time fetching if needed, backups/support exports, and posting through connected social accounts. Do not assume search providers allow reusable media-library imports.
- Provider previews should be accurate enough that operators can catch misleading account, media, or text mismatches before posting.

### Attention, Notifications, And Healthy Use

- Desktop notifications must be useful interruptions: failed publishing, expired credentials, scheduled-post outcomes, and operational health issues.
- Prefer quiet defaults for informational status changes that can be reviewed in-app.
- Do not add infinite feeds, engagement-chasing prompts, or "post more" nudges that are unrelated to operator goals.
- Give operators clear pause, retry, and dismiss controls for background work.

### Access, Inclusion, And Unequal Impact

- Test core workflows at realistic desktop sizes and with keyboard navigation, readable contrast, and clear focus states.
- Do not make safety features, data export, backup, restore, or credential warnings optional premium-style conveniences.
- Make failures understandable for non-developer operators. Error states should explain the action to take without exposing secrets.
- When adding analytics or scoring, avoid presenting provider metrics as moral, personal, or employment judgments about account owners or contributors.

### Automation, Algorithms, And Bias

- Automated jobs must be inspectable through product-level status, logs, and failed-work recovery.
- Do not treat automated provider decisions as automatically correct. Surface provider errors, rate limits, and rejected media rules with enough context for human review.
- Any future AI-assisted writing, media selection, recommendations, scoring, or moderation must disclose that automation is involved and require a separate risk review.
- If analytics summaries or recommendations are added, document inputs, exclusions, and known limitations.

### Surveillance And Sensitive Data

- Keep the local-first architecture: local SQLite, app-owned media storage, OS-backed secrets, and explicit export/backup actions.
- Do not add background telemetry, hidden analytics, or third-party tracking without a product decision, privacy review, and clear operator disclosure.
- Minimize what is stored locally. Store secret references in SQLite, not raw tokens, client secrets, API keys, or refresh tokens.
- Avoid collecting account data that is not needed for publishing, import, reporting, support, or backup/restore.
- Treat subpoenas, device loss, shared Macs, and contractor access as realistic risk scenarios when designing data storage and support workflows.

### Data Control And Monetization

- Operators must be able to understand what local data exists, where the app-data directory is, and how to back it up or restore it.
- Backup files must have manifests and must exclude OS keychain secrets. Restored accounts should require reconnect when secrets are unavailable.
- Do not sell, share, or reuse account data, media, reports, or publishing history outside Dust Wave operations without explicit approval and user-facing policy.
- If a future cloud sync or hosted service is added, document retention, deletion, export, breach response, and account-transfer behavior before implementation.

### User Understanding And Consent

- Avoid hidden behavior. If the app opens a browser, stores credentials, queues jobs, publishes posts, downloads media, or restores a backup, the operator should understand what is happening.
- Keep service setup packets and onboarding exports free of secret values.
- Confirmation dialogs should appear for destructive or externally visible actions: publish now, bulk delete, restore backup, clear logs, disconnect account, and similar operations.
- Terms, support docs, release notes, and in-app copy should be plain-language and should not describe Laravel/server behavior that the desktop app does not use.

### Abuse, Harassment, And Criminal Misuse

- Red-team publishing and account workflows for harassment, impersonation, spam, doxxing, account takeover, coordinated posting, and unauthorized access.
- Do not add bulk publishing or automation features without safeguards for account ownership, review, provider limits, and emergency stop behavior.
- Keep rate-limit deferral and provider-specific restrictions visible enough that operators cannot accidentally overwhelm provider APIs.
- System logs and support exports must redact tokens and secrets while preserving enough diagnostic detail to investigate suspicious activity.
- Document who can revoke credentials, disconnect accounts, stop scheduled publishing, and restore from backup during an incident.

## Release Review Checklist

Before a release candidate is accepted, confirm:

- Provider credential forms save only secret references in local data.
- Account connection, refresh, import, publish, retry, and disconnect flows have clear operator feedback.
- Scheduled publishing can be paused, retried, or recovered without using raw queue controls.
- Failed posts and failed imports preserve actionable errors without exposing secrets.
- Media import, thumbnail generation, deletion, and orphan cleanup do not remove unmanaged user files.
- Backup and restore were tested on a clean app-data directory.
- Desktop notifications interrupt only for meaningful operational events.
- New or changed UI copy is specific to Dust Wave Social and does not describe irrelevant Laravel/server mechanics.
- Product logs, support exports, setup packets, and onboarding packets redact access tokens, refresh tokens, client secrets, and API keys.
- The team has reviewed how the release could be misused for misinformation, harassment, spam, surveillance, or account takeover.

## Feature Design Checklist

Use this checklist in feature specs, issues, or pull requests:

- What data does this feature collect, create, import, export, or delete?
- Is each data field necessary for the operator's workflow?
- What could go wrong if the data were leaked, corrupted, restored to the wrong machine, or accessed by the wrong operator?
- Could this feature publish, schedule, amplify, or automate content in a way that creates public harm?
- Are all externally visible actions preceded by review, confirmation, or a reversible draft state?
- How does the feature behave when credentials expire, providers rate-limit requests, the app closes mid-job, or the network fails?
- What does the operator need to know that is not obvious from the UI?
- What logs or health states are needed for support without exposing secrets?
- What manual operational policy is required outside the app?

## Red-Flag Pathway

Any contributor or operator should stop and flag a release if they find:

- Secret values in logs, exports, setup packets, onboarding packets, backups, screenshots, or crash reports.
- A way to publish externally visible content without sufficient operator intent.
- A way to bulk schedule, duplicate, or retry posts that could bypass provider-specific restrictions.
- Misleading previews, account labels, or status states that could cause posting from the wrong account.
- Hidden data collection, telemetry, or third-party sharing.
- Destructive actions without review or recovery.
- A workflow that materially helps impersonation, harassment, doxxing, spam, or misinformation.

Flagged issues should be recorded in the issue tracker or release notes with an owner, decision, mitigation, and ship/no-ship status. If the issue is a security vulnerability, follow `SECURITY.md` instead of public issue discussion.

## Ongoing Practices

- Keep the Ethical OS risk review in the finish-line checklist for every public release.
- Assign owners for provider credentials, Apple credentials, release approval, backup policy, and emergency account disconnects.
- Revisit this document after major provider API changes, new automation features, AI-assisted features, cloud sync, team collaboration, or new supported social networks.
- Treat "full Mixpost parity" as the baseline, not the ceiling, for safe operations.
