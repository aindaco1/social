# Dust Wave Social User Flows

Updated: 2026-08-22

This is the canonical operator-flow inventory for the local desktop product. `FEATURES.md` defines the supported feature surface; this document defines how an operator reaches an outcome, the UX state verified in the 2026-08-22 audit, and the regression guard for that journey.

## Status key

- **Healthy**: the locally testable path has a clear entry point, state, result, and recovery message.
- **Fixed**: the audit found a confusing or broken step and the current source includes the correction.
- **Manual gate**: the local path is covered up to the safe boundary, but final acceptance needs real provider credentials, a signed release, external network access, or human output review.

Every ID below has a matching test in `test/desktop-user-flows.test.mjs`. The source-contract tests protect the UI entry point and safety/feedback wiring; Rust and Worker suites protect command and service behavior. Manual gates remain release acceptance work and are not represented as completed by an automated test.

## Contextual editing assessment

- **Implemented:** post details can continue into the composer without losing the Calendar or Dashboard origin; post scheduling/retry-time fields open only in the affected library row; label edits remain inside the affected row.
- **Shared behavior:** label and scheduling edits use one `ContextualEditor` for autofocus, Escape-to-cancel, focus return, busy state, and Save/Cancel layout.
- **Already contextual:** connected-account operations, provider-specific composer versions, and media/label selection already act on the object in view and remain unchanged.
- **Intentionally explicit:** OAuth and service credentials, app settings, publishing now, disconnect/delete/restore actions, and media processing retain dedicated surfaces or confirmations because they are consequential, multi-step, or not supported by an update contract.

## Core workspace and dashboard

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| CORE-01 | Launch the app, load local data, and see a usable workspace or an actionable load error. | Healthy. Local-first state and load failures are explicit. | Packaged launch remains part of release smoke testing. |
| CORE-02 | Move among Dashboard, Posts, Calendar, Media, Connections, Analytics, Labels, Settings, and System. | Fixed. Eleven flat destinations were reduced to nine grouped destinations without removing features: Connections contains Connected accounts and Provider setup, while Settings contains Local identity. The active destination exposes `aria-current`. | Automated. |
| DASH-01 | Review connected, scheduled, published, failed, upcoming, provider, and attention summaries. | Fixed. System-health issues such as missing active credentials now surface in workspace attention instead of only changing the System badge. | Automated with data-state coverage in Rust. |
| DASH-02 | Select a connected account and 7-, 30-, or 90-day analytics period. | Healthy. Account and period selection expose selected state and loading/error/empty results. | Real metrics require a connected provider account. |

## Posts and publishing

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| POST-01 | Open Posts > Compose, enter content, and save a draft. | Fixed. Compose and Post library are now separate modes, so drafting is not crowded by filters and history. Save is disabled until the draft is valid and feedback remains in context. | Automated command behavior plus local UI contract. |
| POST-02 | Choose destination accounts and create account-specific copy versions. | Healthy. Account selection, base copy, provider version tabs, and character counts are grouped together. | Provider limits are validated locally; final rules remain provider-dependent. |
| POST-03 | Add labels, emoji, uploaded media, or permitted external media to a draft. | Healthy. Selection counts and removable chips make the draft contents reviewable. | External media terms and live downloads remain provider gates. |
| POST-04 | Recover an unsaved composer draft after leaving or restarting. | Healthy. Local autosave, restore, and discard paths are explicit and restore is cleared after backup restore. | Automated local-storage contract. |
| POST-05 | Validate a post and review provider previews before scheduling. | Healthy. Validation errors are account-specific and previews show destination identity, text, and media. | Live provider validation can still reject content after local validation. |
| POST-06 | Pick a future time and schedule a draft. | Healthy. Timezone-aware scheduling is visible and requires a selected account and valid content. | Live publication is a real-provider acceptance gate. |
| POST-07 | Publish immediately using Post Now. | Healthy. A modal names the selected accounts and requires explicit confirmation before work is queued. | Manual gate; no live post was sent during the audit. |
| POST-08 | Open Posts > Post library; browse, search, filter, select, and paginate posts. | Fixed. The library has a dedicated mode; empty results display `0–0 of 0` instead of the broken `0–-1 of 0`; status tabs expose selected state. | Automated. |
| POST-09 | Open detail, edit, duplicate, validate, or delete an existing post. | Fixed. Post details now offer Edit in composer wherever the detail opened, and the composer names and links back to the originating view. Delete copy explains that queued publishing is cancelled. | Automated command behavior plus contextual-editing UI contract; destructive UI was not executed against operator data. |
| POST-10 | Retry a failed post now or at a chosen time. | Fixed. Retry Now confirms that the action may publish to every assigned account. Schedule and retry-time fields now open on demand in the affected row instead of crowding every post. | Automated contextual-editing contract; still requires a controlled provider test post for end-to-end publishing. |

## Calendar

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| CAL-01 | Move to previous, today, or next and switch Month, Week, or Day. | Fixed. The duplicate range selector was removed; one tab set now controls range and exposes selected state. | Automated. |
| CAL-02 | Filter by date, status, keyword, account, and label. | Fixed. Filters now have explicit accessible names and retain a clear-filter path. | Automated. |
| CAL-03 | Start a new post from a date, day cell, or weekly time slot. | Healthy. The composer opens with the chosen schedule prefilled. | Automated source/command contract. |
| CAL-04 | Open post detail from the calendar, continue editing, or bulk-delete selected posts. | Fixed. Detail preserves status/account context, Edit in composer carries the Calendar origin into the editor, and the editor provides a direct return path. Deletion remains confirmed. | Automated navigation and contextual-editing contract; destructive action was not executed against operator data. |

## Media

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| MEDIA-01 | Import one or more local files by picker, path, or drag and drop. | Fixed. Inputs now have explicit accessible names; per-file results and failures remain visible. | File picker and packaged sidecars need packaged-app smoke coverage. |
| MEDIA-02 | Download media from a URL into the app-owned library. | Fixed. URL, display name, and source label now have unambiguous accessible names. | Network download remains an external acceptance boundary. |
| MEDIA-03 | Search/filter uploaded media, select items, clean or delete app-owned files. | Fixed. Delete confirmation states that only the app-owned copy is removed and that outside originals are untouched. | Destructive action was not executed against operator data. |
| MEDIA-04 | Create a new post from selected uploaded or external media. | Healthy. Selection state and provider policy notes remain visible before opening the composer. | Provider-specific publication remains a manual gate. |
| MEDIA-05 | Search Unsplash stock images and download allowed results. | Healthy. Missing service configuration produces an in-context error rather than a blank result. | Requires an active Unsplash credential and terms acceptance. |
| MEDIA-06 | Search KLIPY GIFs and attach provider references without prohibited permanent storage. | Healthy. Attach-only items are labeled with the policy restriction. | Requires an active KLIPY credential and live provider check. |
| MEDIA-07 | Enable Local AI Media Labs, probe runtime, preflight, draft alt text, upscale, crop, search, cancel, and review derivatives. | Healthy locally. Original preservation, progress, cancellation, and derivative metadata are explicit. | Manual gate for offline packaged runtime and human image/alt-text quality review. |

## Accounts and provider onboarding

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| ACCT-01 | Open Connections > Connected accounts, copy an intake CSV or onboarding plan, and open Add Account. | Fixed. The modal first presents four clear provider choices, then displays only the selected provider's steps. Choice cards use two columns at desktop widths and one column at compact widths; the selected form remains scrollable. | Automated layout contract plus native-app visual check. |
| ACCT-02 | Choose X, start OAuth, authorize in the browser, paste the code, and connect. | Healthy up to authorization. Missing service setup routes to Connections > Provider setup and errors remain provider-specific. | Manual gate requiring a real X account and app credentials. |
| ACCT-03 | Start Meta OAuth, list Pages/Instagram accounts, choose destinations, and connect. | Healthy up to authorization. Facebook Pages and Instagram are explicitly selectable and saved separately. | Manual gate requiring real Meta assets and app review state. |
| ACCT-04 | Register a Mastodon server app, authorize, and connect the returned code. | Healthy up to authorization. Registration and account connection are separate, labeled steps. | Manual gate against an approved Mastodon server. |
| ACCT-05 | Open TikTok broker authorization and connect an assisted-publishing/analytics account. | Healthy up to authorization. Missing client key, active service, HTTPS broker URL, credential, or scopes are called out; direct publishing limits are explicit. | Manual gate requiring the broker and a real TikTok account. |
| ACCT-06 | Refresh, import now, queue imports, or disconnect a connected account. | Fixed. The destructive action is now named Disconnect and explains that local history remains while publishing/imports stop. | Refresh/import require live credentials; disconnect was not executed against operator data. |

## Connections, analytics, labels, and settings

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| SVC-01 | Open Connections > Provider setup; choose Facebook, Instagram Local Media, X, TikTok, Unsplash, or KLIPY; review credential readiness and save credentials/configuration and Active state once. | Fixed. Readiness now lives with the selected provider: tabs show which services need setup, the provider header reports its credential count, and missing-field guidance sits beside the field that needs attention. The duplicate all-provider diagnostic matrix is gone. Ready providers collapse to an Edit Settings action while incomplete providers stay open for repair. | Live credential validity remains a provider gate. |
| SVC-02 | Open setup/docs or copy one field, one provider packet, all setup, or only missing setup without secrets. | Fixed. Batch copy actions now share one Share setup disclosure, while provider-specific setup stays with the selected provider. Copied packets explicitly exclude secret values and direct the operator back to the named Connections tabs. | Automated redaction contracts plus operator review. |
| SVC-03 | Open Connections > Provider setup > Instagram Local Media, paste an expiring one-time setup code, and pair this Mac. | Fixed. Ordinary users no longer need Cloudflare or a reusable shared token. The app exchanges the code for a device-specific credential, stores it in Keychain, activates the service, and then shows a clear paired state. Direct token, Worker, and service URL controls remain available behind Advanced Manual Setup as recovery paths. | Automated Worker and desktop contracts; live pairing is verified during deployment. |
| RPT-01 | Open Analytics, choose account and period, and review provider cards and audience history. | Fixed. Navigation and heading now use one Analytics label; an empty workspace shows disabled, named selectors and an Add Account action instead of a blank pop-up. | Real report values require imported provider data. |
| TAG-01 | Open Labels; create, edit, color-code, assign, filter by, and delete labels. | Fixed. Label editing stays in its row and now reuses the shared contextual editor, with autofocus, Escape-to-cancel, focus return, and consistent Save/Cancel actions. Delete copy explains that posts remain. | Automated command behavior and accessibility contract. |
| SET-01 | Store operator email under Local identity, enable desktop notifications, and send a test alert. | Fixed. Identity is stored once and Notifications references it instead of repeating the email input; test feedback remains in context. | macOS notification permission requires packaged-app acceptance. |
| SET-02 | Opt into or out of Local AI Media Labs. | Healthy. Local-only processing is off by default and tools stay hidden until enabled. | Packaged offline acceptance remains manual. |
| SET-03 | Set timezone, date/time formats, first weekday, and default composer accounts. | Healthy. Settings are grouped by outcome and saved explicitly. | Automated settings persistence plus packaged smoke. |
| PROF-01 | Open Settings > Local identity, save the local operator name/email, and understand workspace security. | Fixed. The separate Profile destination was merged into Settings. App Lock copy directs operators to macOS account/device locking instead of implying an unavailable setup flow. | Automated. |

## System, recovery, and updates

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| SYS-01 | Review unauthorized accounts, failed posts/jobs, queued/processing work, provider limits, service credentials, and media tools. | Fixed. Blocking health issues are visible both on System and in workspace attention. | Automated summary behavior plus packaged media-tool smoke. |
| SYS-02 | Run maintenance or open More actions to clear resolved state, recover stale jobs, or retry failed imports. | Fixed. The primary health action remains visible while lower-frequency actions are disclosed on demand. Background checks stay quiet when nothing changes; a manual no-op reports one concise result. Clear Resolved State explains exactly what it removes in an in-app confirmation. | Recovery commands are covered; destructive confirmation was not accepted in the audit. |
| SYS-03 | Open System logs to refresh, export, or clear redacted logs; use More actions to copy support info or the app-data path. | Fixed. Technical/recovery sections are collapsed until requested. Clear uses the shared confirmation dialog and support exports are designed to redact secrets. | Export redaction remains part of release acceptance. |
| SYS-04 | Create a local backup containing database, app-owned media, and manifest without Keychain secrets. | Healthy. Scope and latest backup details are visible. | Manual packaged backup acceptance. |
| SYS-05 | Choose a Dust Wave backup, create a safety backup, restore, reload, and reconnect credentials. | Healthy. Restore is disabled without a path and requires explicit confirmation describing replacement. | Manual gate on an isolated app-data directory. |
| SYS-06 | Check for a signed GitHub release, download, verify, install, and restart. | Healthy locally. Status/progress/error remain visible and updater resources avoid Vue proxying. | Manual signed/stapled update-path acceptance. |

## Cross-cutting accessibility

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| A11Y-01 | Navigate by keyboard and assistive technology while identifying current page, selected tabs, dialogs, fields, progress, errors, and destructive intent. | Fixed. Grouped navigation exposes the current page; shared workspace, calendar, service, and media tabs expose selection; the shared confirmation dialog exposes modal semantics, focuses on open, and cancels with Escape; key fields have explicit names. | Automated semantic contract plus native keyboard verification and manual VoiceOver release pass. |

## Release acceptance still required

The audit deliberately stopped before creating OAuth grants, sending posts, deleting operator data, restoring a backup, installing an update, or transmitting credentials. Before release, exercise the manual gates above with controlled test accounts and an isolated app-data directory, then record the signed/stapled build version, provider response, and visible result. A locally rendered screen or passing source contract is not evidence that an external provider accepted the operation.
