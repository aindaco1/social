# Dust Wave Social User Flows

Updated: 2026-09-05

This is the canonical operator-flow inventory for the local desktop product. `FEATURES.md` defines the supported feature surface; this document defines how an operator reaches an outcome, the UX state established in the 2026-08-22 audit and subsequent fixes, and the regression guard for that journey.

## Status key

- **Healthy**: the locally testable path has a clear entry point, state, result, and recovery message.
- **Fixed**: the audit found a confusing or broken step and the current source includes the correction.
- **Manual gate**: the local path is covered up to the safe boundary, but final acceptance needs real provider credentials, a signed release, external network access, or human output review.
- **Needs work**: a current native interaction or source inspection found an unresolved usability or correctness gap, even when the wiring tests pass.

Every ID below has a matching test in `test/desktop-user-flows.test.mjs`. The source-contract tests protect the UI entry point and safety/feedback wiring; Rust and Worker suites protect command and service behavior. Manual gates remain release acceptance work and are not represented as completed by an automated test.

The September review supersedes older general health claims where noted below. See [UX_REVIEW.md](UX_REVIEW.md) for current native evidence, the Mixpost comparison, prioritized findings, and the distinction between a source contract and end-to-end acceptance. Theme and modal-focus behavior additionally have executable tests in `test/desktop-appearance.test.mjs` and `test/desktop-dialog-focus.test.mjs`.

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
| POST-01 | Open Create post or Posts > Compose, enter content, and save a draft. | Fixed. Explicit Save keeps the editor open with Saved to Post library feedback; recovery status is separate. Wide layouts place the reused preview beside writing, and repeated version/library controls are disclosed as needed. | Native isolated save and keep-editing checks passed; provider publication remains separate. |
| POST-02 | Choose destination accounts and create account-specific copy versions. | Healthy. Account selection, base copy, provider version tabs, and character counts are grouped together. | Provider limits are validated locally; final rules remain provider-dependent. |
| POST-03 | Add labels, emoji, uploaded media, or permitted external media to a draft. | Fixed locally. Selection counts and removable chips make contents reviewable. Emoji Escape returns to its trigger; selection restores editor focus without inserting an extra newline. | Native emoji search/Enter/Undo/Escape and local-media Tab/Space selection passed. The draft's media assignment persisted in SQLite with no queued publication. External media terms and live downloads remain provider gates. |
| POST-04 | Recover an unsaved composer draft after leaving or restarting. | Fixed. Shared replacement protection offers Keep editing, Save draft and continue, or explicit discard. Initial loading cannot overwrite the recovery buffer; storage failures remain visible. | Executable decision/concurrency tests and native keep/save-and-continue checks; recovery is a device-local copy, not a backup. |
| POST-05 | Validate a post and review provider previews before scheduling. | Healthy. Validation errors are account-specific and previews show destination identity, text, and media. | Live provider validation can still reject content after local validation. |
| POST-06 | Pick a future time and schedule a draft. | Fixed locally. Scheduling uses the saved workspace timezone, rejects nonexistent/repeated new wall times, preserves existing instants, and stops if saving fails. The app-open/awake/online requirement stays visible. | Cross-zone/DST behavior tests pass; real provider delivery remains a manual gate. |
| POST-07 | Publish immediately using Post Now. | Healthy. A modal names the selected accounts and requires explicit confirmation before work is queued. | Manual gate; no live post was sent during the audit. |
| POST-08 | Open Posts > Post library; browse, search, filter, select, and paginate posts. | Fixed. The library has a dedicated mode and correct empty count. Row actions now wrap beneath the record rather than sitting beyond a wide horizontal scroll. | Source contract plus native saved-draft visual check; dense multi-account rows need minimum-width acceptance. |
| POST-09 | Open detail, edit, duplicate, validate, or delete an existing post. | Fixed. Post details now offer Edit in composer wherever the detail opened, and the composer names and links back to the originating view. Delete copy explains that queued publishing is cancelled. | Automated command behavior plus contextual-editing UI contract; destructive UI was not executed against operator data. |
| POST-10 | Retry a failed post now or at a chosen time. | Fixed. Retry Now confirms that the action may publish to every assigned account. Schedule and retry-time fields now open on demand in the affected row instead of crowding every post. | Automated contextual-editing contract; still requires a controlled provider test post for end-to-end publishing. |

## Calendar

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| CAL-01 | Move to previous, today, or next and switch Month, Week, or Day. | Fixed. Rust owns one aligned date window and UTC query interval; the UI renders it and loads all pages. Month navigation handles month-end dates and calendar date/post buttons are separate targets. | Automated month, leap-year, DST, week-start, first/last-instant tests; native September grid now includes October 11. |
| CAL-02 | Filter by date, status, keyword, account, and label. | Fixed. Filters now have explicit accessible names and retain a clear-filter path. | Automated. |
| CAL-03 | Start a new post from a date, day cell, or weekly time slot. | Fixed. Shared draft protection runs before replacement. A past shortcut suggests the next future quarter-hour at least 30 minutes away; the chosen timezone and resulting date/time are explicit. | Native Keep editing and Save draft and continue passed; future-time and DST cases have behavioral tests. |
| CAL-04 | Open post detail from the calendar, continue editing, or bulk-delete selected posts. | Fixed. The agenda pages 25 rows at a time while the grid retains every queried post. Month overflow opens the full day; selections are named. Detail and Edit in composer preserve Calendar context. Deletion remains confirmed. | Native traversal of all nine pages in a 205-post day and opening post 205 passed; automated paging/navigation contracts. No destructive action against operator data. |

## Media

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| MEDIA-01 | Import one or more local files by picker, path, or drag and drop. | Fixed. Picker/drop lead the page; path and URL entry are secondary disclosures. Inputs have explicit accessible names; per-file results and failures remain visible. | Native local import and compact Light layout checked; packaged sidecars need release smoke coverage. |
| MEDIA-02 | Download media from a URL into the app-owned library. | Fixed. URL, display name, and source label now have unambiguous accessible names. | Network download remains an external acceptance boundary. |
| MEDIA-03 | Search/filter uploaded media, select items, clean or delete app-owned files. | Fixed. Delete confirmation states that only the app-owned copy is removed and that outside originals are untouched. | Destructive action was not executed against operator data. |
| MEDIA-04 | Create a new post from selected uploaded or external media. | Healthy. Selection state and provider policy notes remain visible before opening the composer. | Provider-specific publication remains a manual gate. |
| MEDIA-05 | Search Unsplash stock images and download allowed results. | Healthy. Missing service configuration produces an in-context error rather than a blank result. | Requires an active Unsplash credential and terms acceptance. |
| MEDIA-06 | Search KLIPY GIFs and attach provider references without prohibited permanent storage. | Healthy. Attach-only items are labeled with the policy restriction. | Requires an active KLIPY credential and live provider check. |
| MEDIA-07 | Enable Local AI Media Labs, probe runtime, preflight, draft alt text, upscale, crop, search, cancel, and review derivatives. | Implemented locally. Shared feedback and worker/runtime ownership; terminating cancellation before native commit; editable/discardable alt-text drafts; background native media work. Packaged fixes cover CSP, scoped canvas input, typed pixels, JSPI-aware CPU fallback, and explicit bundled Wasm location. | Signed ux.7 passed live CPU cancellation with no output, retry without restart, navigation and composer input during inference, and a verified 32px → 128px derivative. Its successful retry measured 155.4s processing and a 2105ms maximum UI timer gap; performance, offline operation, full VoiceOver, and representative photo quality remain open. ux.5 separately preserved derivative metadata through backup/restore. See the background model processing evidence in UX_REVIEW.md for exact scope and initial control failures. |

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
| SVC-01 | Open Connections > Provider setup; choose Facebook, Instagram Local Media, X, TikTok, Unsplash, or KLIPY; follow the guided setup path; review credential readiness; and save credentials/configuration and Active state once. | Fixed. Each selected provider now presents one ordered portal-to-test-account checklist, exact setup actions, credential progress, and the next locally verifiable step. Tabs show which services need setup, missing-field guidance sits beside the field that needs attention, ready providers collapse to Edit Settings, and the shared catalog drives both the UI and setup guide. | Live credential validity and provider-side completion remain provider gates. |
| SVC-02 | Open setup/docs or copy one field, one provider packet, all setup, or only missing setup without secrets. | Fixed. Batch copy actions now share one Share setup disclosure, while provider-specific setup stays with the selected provider. Copied packets explicitly exclude secret values and direct the operator back to the named Connections tabs. | Automated redaction contracts plus operator review. |
| SVC-03 | Open Connections > Provider setup > Instagram Local Media, paste an expiring one-time setup code, and pair this Mac. | Fixed. Ordinary users no longer need Cloudflare or a reusable shared token. The app exchanges the code for a device-specific credential, stores it in Keychain, activates the service, and then shows a clear paired state. Direct token, Worker, and service URL controls remain available behind Advanced Manual Setup as recovery paths. | Automated Worker and desktop contracts; live pairing is verified during deployment. |
| RPT-01 | Open Analytics, choose account and period, and review provider cards and audience history. | Fixed locally. Missing measurements stay No data/gaps while measured zero stays zero. Both report views share latest-observation status and contextual import feedback. Summary averages name observed days; late responses cannot replace the selected report. | Rust/JS partial-observation and zero tests plus native synthetic reports; live freshness, scopes, and import failures remain provider gates. |
| TAG-01 | Open Labels; create, edit, color-code, assign, filter by, and delete labels. | Fixed. Label editing stays in its row and now reuses the shared contextual editor, with autofocus, Escape-to-cancel, focus return, and consistent Save/Cancel actions. Delete copy explains that posts remain. | Automated command behavior and accessibility contract. |
| SET-01 | Store operator email under Local identity, enable desktop notifications, and send a test alert. | Fixed. Identity is stored once and Notifications references it instead of repeating the email input; test feedback remains in context. | macOS notification permission requires packaged-app acceptance. |
| SET-02 | Opt into or out of Local AI Media Labs. | Healthy. Local-only processing is off by default and tools stay hidden until enabled. | Packaged offline acceptance remains manual. |
| SET-03 | Set timezone, date/time formats, first weekday, and default composer accounts. | Fixed. One shared parser/formatter uses workspace settings across composer, calendar, library, details, and reporting dates. Existing UTC schedules are unchanged. | Native UTC-to-Denver conversion with ISO/24-hour display passed; executable offset, DST, SQLite-UTC, and date-boundary tests. |
| SET-04 | Open Settings > Appearance and choose System, Light, or Dark. | Fixed. Defaults to System, responds to macOS appearance changes, persists explicit overrides, and updates shared CSS/chart colors immediately without Save Settings. | Behavioral system/persistence/contrast tests and native Light/Dark/relaunch checks; signed-release acceptance remains separate. |
| PROF-01 | Open Settings > Local identity, save the local operator name/email, and understand workspace security. | Fixed. The separate Profile destination was merged into Settings. App Lock copy directs operators to macOS account/device locking instead of implying an unavailable setup flow. | Automated. |

## System, recovery, and updates

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| SYS-01 | Review unauthorized accounts, failed posts/jobs, queued/processing work, provider limits, service credentials, and media tools. | Fixed locally. Health diagnostics run off the UI thread; per-tool availability probes time out and clean up the child. Actual tool results and recovery instructions wrap fully. | Automated summary/timeout/reaping tests and native unresponsive-tool injection passed. The isolated signed candidate reported both tools available from its bundle on this Mac; clean-Mac acceptance remains open. |
| SYS-02 | Run maintenance or open More actions to clear resolved state, recover stale jobs, or retry failed imports. | Fixed. The primary health action remains visible while lower-frequency actions are disclosed on demand. Background checks stay quiet when nothing changes; a manual no-op reports one concise result. Clear Resolved State explains exactly what it removes in an in-app confirmation. | Recovery commands are covered; destructive confirmation was not accepted in the audit. |
| SYS-03 | Open System logs to refresh, export, or clear redacted logs; use More actions to copy support info or the app-data path. | Fixed. Technical/recovery sections are collapsed until requested. Clear uses the shared confirmation dialog and support exports are designed to redact secrets. | Signed candidate export excluded the test image filename, hashes, AI metadata, embeddings, and alt text. No real credentials were present; live-secret acceptance remains separate. |
| SYS-04 | Create a local backup containing database, app-owned media, and manifest without Keychain secrets. | Healthy. Scope and latest backup details are visible; long paths wrap. | Debug fixture backup passed previously. Signed 0.1.10-ux.5 backed up the original, model derivative, thumbnails, and metadata: two records/four files, matching hashes, valid SQLite, and an explicit Keychain exclusion. |
| SYS-05 | Choose a Dust Wave backup, create a safety backup, restore, reload, and reconnect credentials. | Healthy locally. Restore requires an explicit replacement confirmation and creates a safety backup. Result paths use readable full-card rows; restored totals include the database, not only media. | Debug and signed isolated restores passed. In 0.1.10-ux.5 the disposable label survived only in the safety copy; three databases passed integrity checks, all four file hashes matched, and derivative metadata survived. No credentials or jobs were present. Live credential reconnection and representative production-data migration remain release gates. |
| SYS-06 | Open the app to check the signed GitHub feed quietly, or check on demand; review an available release, then explicitly download, verify, install, and restart. | Fixed. Launch discovery reuses the manual signed-update path and stays out of the top bar when current or temporarily unavailable. Available releases surface in the existing controls; updater resources remain outside Vue deep proxies. | Automated launch/check contracts; the signed/stapled install-and-restart path remains a manual release gate. |

## Cross-cutting accessibility

| ID | Operator outcome and path | UX audit result | Acceptance boundary |
| --- | --- | --- | --- |
| A11Y-01 | Navigate by keyboard and assistive technology while identifying current page, selected tabs, dialogs, fields, progress, errors, and destructive intent. | Improved, not fully accepted. Shared dialogs recover focus when a step removes its button; tabs retain one Tab stop and manual activation. Post editor, post/media selections, and restore input have names. Emoji selection/dismissal restores the appropriate focus. Tauri's built-in ⌘+/⌘−/⌘0 zoom is enabled; compact reflow keeps the composer action bar in document flow. | Behavioral focus/tab tests and native provider-step Back/Escape, emoji search/selection/Undo/Escape, and local-media Tab/Space passed. Five-provider long-name selection, 200% Dark Settings/Light editor reflow, and reset-to-100% passed locally. VoiceOver was enabled with permission but speech/cursor feedback could not be reliably observed through this session; it was turned off afterward. Full spoken-flow, nested-picker screen-reader, and error-announcement acceptance remain open. |

## Release acceptance still required

The audit deliberately stopped before creating OAuth grants, sending posts, deleting operator data, installing an update, or transmitting credentials. Backup restore was exercised only in the approved disposable workspace. The user chose to stop at local validation: no new packaged candidate, notarization, installation, publication, or live-provider test is authorized by this continuation. Before release, exercise the remaining manual gates above with controlled test accounts and isolated app data, then record the signed/stapled build version, provider response, and visible result. A locally rendered screen or passing source contract is not evidence that an external provider accepted the operation.
