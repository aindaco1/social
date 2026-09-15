# Dust Wave Social: project status and completion plan

Updated: 2026-09-15. Audience: project owner, engineering, and acceptance testers.
Implementation baseline reviewed: `6e3014973063e91d2b095cb27d9b7b93c872ed07`.

This is the single maintained source for current status, remaining work, priorities,
and acceptance evidence. It combines the project-status summary and detailed
completion plan, retaining the newer September 15 findings. The guides linked
below continue to own exact setup, build, release, and recovery procedures.

The attached documents are source material. Their unfinished checklists describe
future work; consolidating and publishing this guide does not execute those tasks.

Quick navigation: [current state](#where-the-project-stands),
[evidence](#evidence-boundaries), [integrations](#integration-acceptance),
[work packages](#prioritized-next-steps), [execution order](#recommended-execution-order),
[sign-off](#final-sign-off-checklist), and [maintenance](#documentation-and-maintenance).

## Summary

The desktop foundation is built and 0.1.10 is published. The remaining work is a
dependency update, resolution of the recorded font-distribution gate, a small
Instagram cleanup gap, and evidence that the released product works with real
accounts, representative data, assistive technology, and other Macs.

The attached August 29 MP4 is available and intact. It can supply a controlled
TikTok test post. A separate recording of Social's real authorization and analytics
flow is still needed for the documented TikTok review path.

## Where the project stands

[Social 0.1.10](https://github.com/aindaco1/social/releases/tag/v0.1.10) is the
published Apple Silicon macOS release; 0.1.9 is the recorded rollback baseline.
The desktop foundation is implemented. Broad user, provider, accessibility,
other-Mac, and full-app offline acceptance remains open.

| Goal | Implemented / demonstrated | Still needed |
| --- | --- | --- |
| Local-first publishing without a Laravel server | Tauri/Vue/Rust, SQLite, app-owned media, Keychain secrets, app-open durable jobs, drafts/previews/scheduling/retry | Real provider delivery and recovery on controlled accounts; the Mac must remain open, awake, and online for scheduled work |
| Simple, accessible UX informed by Mixpost | System/Light/Dark, compact shared controls, focused composer and adjacent preview, progressive setup, draft protection, consistent timezone/calendar and missing-data handling | Full spoken VoiceOver flows, dense multi-account/long-label review on the released build, and another-user onboarding |
| Useful LiteRT upscaling | Bundled native LiteRT 2.1.6 CPU on macOS 14+, older-system Wasm compatibility, 4× derivatives, progress/cancel/retry, original/alpha preservation | Other-Mac compatibility/performance, human image-quality approval, and full-app offline acceptance |
| Easier platform integrations | One catalog-driven setup flow, one save action, exact secret-free setup packets, per-server Mastodon registration, one-use Instagram media pairing | Fresh personal-account end-to-end results and provider approvals needed for unrelated users |
| Reliable recovery and distribution | Official signed updater hop; isolated backup/restore and redacted exports tested | Representative user-data/Keychain survival and independent clean-Mac installation |

## Evidence boundaries

The 0.1.10 release workflow passed 156 JavaScript tests, 150 Rust tests, and both
six-test Worker suites, then packaged-launch and native-helper offline checks.
Those are release engineering results, not a user study or provider acceptance.

The last isolated native UI review established:

- Draft keep/save-and-continue, explicit-save feedback, timezone conversion,
  missing-versus-zero charts, compact dropdowns, and the 205-post calendar agenda.
- Keyboard provider steps, emoji selection/Undo/Escape, media selection, and
  200% reflow at the supported 1100×720 minimum window.
- In the final notarized UX candidate, a 512px photograph saved as a 2048px
  derivative in 22.7 seconds; cancellation/retry, navigation/typing, transparency,
  oversize rejection, original preservation, and derivative metadata passed.
- Packaged isolated recovery restored 30 media files byte-for-byte, preserved
  derivative metadata, created a safety backup, and passed support-log checks.

These historical results are not new measurements on every Mac. Earlier Wasm
timings varied substantially and must not be presented as a measured native
speedup. Full VoiceOver speech/cursor feedback was not reliably observable; that
gate remains open. No production account credentials were present in the isolated
recovery tests.

Include representative MP4 import/preview and bundled-media-tool behavior in the
released-build pass. Still-image upscaling does not validate video ingestion.

The released helper passed TCP/UDP IPv4/IPv6 denial controls, reference-identical
inference, cancellation/reaping, and malformed-frame rejection both in CI and from
an independent download. Its report intentionally says
`full_app_offline_accepted: false`. The previous whole-app sandbox/LuLu attempts
still allowed fresh WebKit network traffic. See [the offline procedure](LOCAL_AI.md#packaged-app-acceptance).

## Integration acceptance

Use personal/test identities first (`aindaco1` / `alonso_in` where applicable),
including the Facebook test Page. Do not onboard Dust Wave organization accounts
or expand provider permissions merely to mark a checklist complete.

The September 5 UI review observed stored X, Facebook Page, Mastodon, and TikTok
account records; Instagram was not connected. This is historical configuration
evidence, not a fresh credential or live publishing check.

| Integration | Evidence / present boundary | Next concrete check |
| --- | --- | --- |
| X | Adapter and stored-account evidence; API tier governs allowed operations | Refresh the personal account; verify a controlled post, scheduled post, import, and retry without duplicates |
| Facebook Page | Shared Meta setup and test-Page record | Verify the login configuration and minimal required permissions; refresh/import, then explicitly approve a test Page post |
| Instagram | Professional-account and one-use staging paths implemented; no connected account in the last UI review | Select an eligible personal/test professional account, pair the Mac, publish one static image, verify deletion of staged media and insights import |
| Mastodon | Per-server registration/OAuth and stored-account evidence | Refresh/import and run a controlled media post plus expected-error recovery on the selected server |
| TikTok | Broker and Assisted mode implemented; stored-account evidence does not prove current Sandbox scopes or Production approval | Revalidate the Sandbox authorization/import, produce a real authorization-and-analytics review demo, then assess Production review |
| Unsplash | Account creation was reported; demo/public access-key setup is documented | Verify search, attribution, and download-trigger behavior in the released app |
| Klipy | The August 28 signed local test returned 18 results with branding | Recheck test-key search/attach flow; verify content filters and production access before broader use |

Use [Provider Setup](PROVIDER_SETUP.md) for exact values and the single provider
acceptance-record format. The attached media inventory is below; CP-04 owns the
remaining TikTok demonstration, and CP-06 owns the Instagram cleanup repair.

The September 6 cleanup's read-only preflight received healthy responses from both
Worker health endpoints. It also reported six configuration warnings: five desktop
provider environment checks (which do not inspect saved Keychain credentials),
and absent `TIKTOK_CLIENT_KEY` / `TIKTOK_CLIENT_SECRET` in the checked environment
and GitHub secret-name inventory. This does not prove the live Worker lacks its
secrets. Before a broker redeploy, reconcile the intended Sandbox/Production
values through the existing private provisioning path without printing them.

## What “complete” means

Use two acceptance milestones:

1. **Supported MVP accepted:** the Apple Silicon desktop app reliably performs
   the workflows in [Features](FEATURES.md), using controlled accounts, with
   documented recovery, accessible operation, accepted media behavior, and a tested
   signed installation/update path. Every intended provider has a dated result.
2. **Ready for wider distribution:** the accepted MVP also has the provider access,
   review approvals, redistribution evidence, onboarding, and service ownership
   needed for people beyond the owner's test accounts.

These milestones do not require adding deferred features. If a supported feature
cannot pass, repair it or make an explicit product decision to narrow the claim and
update the product/docs. An untested feature is not a pass. A provider approval
pending for unrelated users can remain a wider-distribution blocker after the
controlled-account milestone passes.

Status terms in this plan:

- **Verified now:** inspected on September 15, with the boundary stated.
- **Recorded:** supported by earlier repository evidence, not repeated in this audit.
- **Open:** work or acceptance evidence is still needed.
- **Conditional:** required only for a chosen distribution or feature scope.
- **Inconclusive:** the attempted check cannot establish success or failure.

## Audit baseline and attached files

| Area | Evidence as of this audit | What it establishes |
| --- | --- | --- |
| Desktop release | GitHub reports public, non-prerelease `v0.1.10`, published September 6, with five assets; their API digests match the release record. The downloaded latest manifest reports `0.1.10` and `darwin-aarch64`. | The public release exists. Artifact signatures, notarization, and installation were not repeated today. |
| Installed application | Read-only `/Applications/Dust Wave Social.app/Contents/Info.plist` reports `0.1.10`, identifier `com.dustwave.social`. | Installed version only; the app was not launched in this audit. |
| Prior release validation | Release record includes 156 JavaScript tests, 150 Rust tests, two six-test Worker suites, notarization, package checks, and a staged 0.1.9 → 0.1.10 update/relaunch. | Historical release engineering evidence; see [release record](RELEASE_OPERATIONS.md#published-0110-evidence). |
| Source after release | The reviewed implementation baseline adds a dependency from the media-staging deployment job to its test job. GitHub's September 7 manual workflow reports both checks and deployment successful. | The deployment gate is implemented and a deployment succeeded; live pairing/publication still needs acceptance. |
| Dependency maintenance | Lockfile contains Tiptap core/Vue/link `2.12.0`. Fresh production npm audit reports 10 moderate findings in the Tiptap family. | The known dependency work remains open. These are related affected packages, not ten independently demonstrated application exploits. |
| Focused local checks | Documentation check, release-version alignment, and eight editor-boundary/release-documentation/release-note tests passed. | Documentation and the existing scoped editor boundary are consistent; no full Rust/build/provider acceptance suite was rerun. |
| Public site and service health | Automated requests to the product, terms, privacy, and both Worker health URLs returned HTTP 403. The browsing tool could not open the product URLs. | Inconclusive from this environment. Recheck in a normal browser and inspect the deployment if needed; do not call this proof of an outage or valid public access. |
| Work tracking | GitHub returned no open issues or pull requests. The tracked worktree was clean before the September 15 documentation work. | The remaining acceptance work is documented rather than represented by an existing open issue queue. |

Relevant online records: [release](https://github.com/aindaco1/social/releases/tag/v0.1.10),
[media-staging deployment](https://github.com/aindaco1/social/actions/runs/34165247890),
and [upstream Tiptap advisory](https://github.com/ueberdosis/tiptap/security/advisories/GHSA-cp6q-959q-f8rh).
Online status is a dated snapshot and must be rechecked when its work begins.

### Attached asset inventory

| File | Verified properties | Role |
| --- | --- | --- |
| `dust-wave-social-sandbox-test-2026-08-29.mp4` | 91,658 bytes; 6 seconds; 1080×1920; 30 fps / 180 frames; H.264 High, yuv420p; AAC LC stereo, 48 kHz. Full decode with host FFmpeg completed without errors; midpoint frame shows the supplied test card. | Existing test-post media for a controlled TikTok import demonstration. |
| `dust-wave-social-sandbox-test-card.png` | 1,109,625 bytes; 1080×1920; supplied icon and text: “DUST WAVE SOCIAL”, “Sandbox integration test”, “August 29, 2026”. | Source title card and visual reference. |

Current local originals:

```text
/Users/aindaco1/Desktop/dust-wave-social-sandbox-test-2026-08-29.mp4
/Users/aindaco1/Desktop/dust-wave-social-sandbox-test-card.png
```

SHA-256:

```text
MP4: b343491a412fd3ef2725da61d36496b2f95704466ac8ad02d110f4dcae3cd193
PNG: 1fb528a91d10e765cafe80536b82985693e336d4c20be08ab72caff6c9f14c08
```

The MP4 hash matches the August 29 provenance recorded in prior project context.
The repository currently has no `artifacts/` directory; the September 6 reference
to a retained local QA/provenance packet is no longer accurate for this checkout.
Keep these originals and their hashes. Restoring an old packet is optional; making
the same static video again is unnecessary. Neither file proves a TikTok upload,
successful import, reviewer demo, review submission, or approval.

## Prioritized next steps

Owners below are roles to assign, not claims that someone has accepted a task.

| ID | Priority | Work | Owner | Depends on |
| --- | --- | --- | --- | --- |
| CP-01 | P0 | Patch the editor dependency family | Engineering | Current advisory/version review |
| CP-02 | P0 | Close font redistribution and packaging evidence | Owner + release maintainer | License evidence or approved replacement |
| CP-03 | P0 | Other-Mac installation, update, and data survival | Tester + owner | Another Apple Silicon Mac; repeat affected checks on final candidate |
| CP-04 | P1 | Complete TikTok Sandbox and record a real demo | Owner + integration engineer | Browser/portal access; broker credentials; controlled target account |
| CP-05 | P1 | Accept X, Facebook Page, and Mastodon | Owner + integration engineer | Controlled accounts and provider access |
| CP-06 | P1 | Complete Instagram pairing, cleanup, and publication | Engineering + owner | Eligible professional account; Meta setup; staging service |
| CP-07 | P1 | Accept Unsplash and Klipy media workflows | Owner + tester | Test credentials; production access for wider use |
| CP-08 | P1 | Prove live scheduling and recovery | Tester + engineering | Accepted direct-publishing provider(s) |
| CP-09 | P1 | Complete VoiceOver and dense-layout acceptance | Accessibility tester | Exact packaged candidate |
| CP-10 | P1 | Complete Local AI machine, offline, and quality acceptance | Tester + owner | Exact package; suitable hardware and isolated network test |
| CP-11 | P1 | Complete representative media and recovery acceptance | Tester + engineering | Isolated workspace and controlled credentials |
| CP-12 | P1 | Prove another user can onboard | New test user + engineering | Setup and recovery paths stable |
| CP-13 | P2 / distribution gate | Complete provider approvals and service ownership | Owner + service operator | Controlled-account evidence and review demo |
| CP-14 | Final gate | Validate and accept the completion release | Release maintainer + owner | Applicable work above closed or explicitly scoped |

### CP-01 — Patch Tiptap without replacing the editor workflow

**Current evidence:** 0.1.10 retains the affected dependency. The repository's
fixed string/schema/CSP boundary passed its focused regression tests today, but
that mitigation is not a patched library. Upstream lists `3.30.4` as patched;
today's npm audit proposes `3.31.3` as a major update. Select a compatible patched
version after reviewing the migration requirements at implementation time.
[Security policy](../SECURITY.md#0110-dependency-review) records the exposure boundary.

- [ ] Update the coordinated Tiptap packages and lockfile; avoid a forced blanket audit fix.
- [ ] Preserve the shared `Div` schema, stored draft HTML, existing composer, and account-specific versions.
- [ ] Review changed APIs such as content replacement/update events, History, extension exports, and Vue integration before porting calls.
- [ ] Exercise paste, links, emoji insertion, selection/focus, Undo/Redo, draft save/recovery, switching versions, and opening old saved posts.
- [ ] Retain meaningful prototype/event-attribute tests against the actual editor schema and verify rendering in the packaged app.
- [ ] Check the retained Mixpost editor/build too, because it shares the packages and schema extension.
- [ ] Rerun the production audit and full applicable release checks; update the security note with the resolved versions and any residual findings.

**Done when:** the resolved graph no longer contains this advisory, old drafts remain
editable, security/behavior checks pass, and the rendered composer passes review.
If unrelated findings appear, record their actual impact and disposition separately.

**Existing implementation:** [desktop composer](../resources/desktop/src/App.vue),
[shared Div extension](../resources/js/Extensions/TipTap/Div.js),
[legacy editor composable](../resources/js/Composables/useEditor.js),
[editor security tests](../test/desktop-editor-security.test.mjs).

### CP-02 — Close the recorded font-distribution gate

The display font is loaded through the existing CSS import and `--dw-font-display`
token. The preflight checks that the Gambado files exist; it does not establish
redistribution permission. This is an existing project gate, not a new license conclusion.

- [ ] Locate evidence covering distribution of both bundled Gambado font files in the desktop app.
- [ ] Record the permission and required attribution in the maintained notices, with private purchase details kept private.
- [ ] If the evidence is insufficient, have the owner select/approve a replacement and use the existing font token/import.
- [ ] If replacing, check headings, navigation, buttons, long labels, both themes, minimum width, and 200% zoom for changed metrics.
- [ ] Confirm the final package retains the existing FFmpeg, LiteRT/runtime-dependency, model, and Mixpost notices and required redistribution materials.

**Done when:** each distributed font has recorded permission or is replaced with an
approved distributable asset, and the exact package and relevant visual checks pass.

**References:** [font definitions](../resources/css/fonts.css),
[desktop tokens](../resources/desktop/src/styles.css), [notices](THIRD_PARTY_NOTICES.md).

### CP-03 — Validate installation and updates on another Mac

Use the [other-Mac checklist](RELEASE_OPERATIONS.md#testing-on-another-mac).
Begin with the existing official release to identify machine-specific problems;
repeat affected checks on the final patched candidate.

- [ ] Record Mac/chip, macOS, source app version, destination version, and test-data scope.
- [ ] Back up representative data and retain the known-good installer.
- [ ] Exercise the documented 0.1.9 → 0.1.10 update if a suitable existing test installation is available; record the actual relaunch and displayed version.
- [ ] For the next release, test its real previous-release → candidate update separately.
- [ ] Verify saved drafts, versions, labels, media, settings, histories, and provider readiness after the update. Check Keychain-backed account refresh without logging secrets.
- [ ] Test a clean install in a separate macOS account or clean Mac, including Gatekeeper and quit/relaunch.
- [ ] Include MP4 import/preview, bundled media tools, actual AI runtime, and notification permission/results.

**Done when:** both an existing-data update and an independent clean install have
dated pass results, with no unexpected data loss or credential prompts. Same-Mac
backup restoration and cross-Mac reconnection are distinct checks: backups exclude
Keychain material. Do not remove real user data to manufacture a clean install.

### CP-04 — Finish TikTok Sandbox and the real review demonstration

Follow [TikTok setup](PROVIDER_SETUP.md#tiktok) and the
[broker runbook](../workers/tiktok-broker/README.md). Keep Assisted publishing and
the current analytics scope; direct API publishing remains deferred.

- [ ] Open the product, terms, privacy, and broker-health URLs in a normal browser. Resolve any actual public-access issue; today's automated 403s are inconclusive.
- [ ] Inspect the current portal: saved app details, domain verification, Sandbox, target consumer account, products/scopes, and callback. Record what is actually saved.
- [ ] Reconcile the intended Sandbox client key/secret privately between the portal and broker. The desktop gets the matching Client Key only; no client secret.
- [ ] Confirm broker provisioning/redeployment can be reproduced without replacing encryption keys, invalidating existing connections, or printing credentials. Earlier missing environment/GitHub secret-name checks did not establish that deployed secrets were absent.
- [ ] Authorize the controlled target account through the broker, connect in Social, and run Import. Verify displayed profile/audience/video data or a legitimate successful empty result.
- [ ] Determine whether the attached MP4 is already posted. If absent, prepare a concrete controlled-account upload for the owner's approval, then retain the provider result after the approved upload.
- [ ] Import the known test post and verify its identity and available metrics. Empty import success proves an authorization path; it does not prove ingestion of a known video.
- [ ] Test reauthorization/reconnect and a controlled revoked/expired connection error without affecting unrelated accounts.
- [ ] Record a separate screen demo: real Social setup/context → browser authorization → connected account → Import → actual analytics/video result. Exclude credentials and one-time codes from the recording.
- [ ] Verify that the explanation/demo matches every requested product/scope and clearly describes Assisted mode.

**Done when:** a dated Sandbox acceptance record and usable authorization/analytics
demo exist. Submitting Production review and receiving approval are separate
checkpoints under CP-13. The static title-card MP4 satisfies neither checkpoint.

**Regression IDs:** ACCT-05, ACCT-06, SVC-01/02, RPT-01.

### CP-05 — Complete X, Facebook Page, and Mastodon acceptance

Use personal/test identities first (`aindaco1` / `alonso_in` where applicable) and
the controlled Facebook test Page. Stored account records are historical setup
evidence; inspect current authorization before relying on them.

| Provider | Required checks | Provider-specific boundary |
| --- | --- | --- |
| X | OAuth/refresh; controlled text and supported media post; scheduled delivery; audience/post imports; expected-error retry and reconnect | Confirm the account's actual tier supports each attempted operation. Preserve the restriction on identical simultaneous posts to multiple X accounts. |
| Facebook Page | Current Meta login configuration; intended Page selection; refresh/import; controlled post; supported photo/video path; scheduling/recovery | Use only necessary permissions. A personal Facebook identity authorizes a Page; it is not a personal-timeline publishing feature. |
| Mastodon | Per-server registration/OAuth; refresh/import; controlled text/media posting; asynchronous media completion; scheduling/recovery | Record server origin and supported rules. Do not generalize one server's result to all Mastodon servers. |

- [ ] For each provider, use the [acceptance record](PROVIDER_SETUP.md#acceptance-record).
- [ ] Obtain approval for concrete externally visible tests, verify the provider-side post/result, and record cleanup.
- [ ] Record each supported format as pass/fail/not tested. A text-only success is insufficient evidence for advertised video/GIF support.
- [ ] Verify imports display missing measurements and measured zero accurately, including selected date range and freshness.
- [ ] Verify disconnect/reconnect on a controlled account and clear failure guidance.

**Done when:** every intended provider/format has an appropriate dated result and
recovery evidence. If access blocks a test, record the exact blocker and owner.
Do not mark Dust Wave organization accounts accepted based on personal tests;
onboard them later only if they are part of the owner's chosen deployment.

**Regression IDs:** ACCT-02/03/04/06, POST-02/05/06/07/10, DASH-02, RPT-01.

### CP-06 — Finish Instagram pairing, staged-media cleanup, and live publication

The last recorded UI review had no connected Instagram account. The current source
supports one static image through Meta professional-account discovery and managed
media staging. Successful September 7 Worker deployment is separate from this
end-to-end acceptance.

**Source gap found in this audit:** `publish_instagram_target` stages local media
with a 24-hour TTL and records the staged objects on success. The inspected desktop
path has no immediate deletion call after either publication success or failure.
The Worker already implements authenticated `DELETE /media/:key` plus expiration
and scheduled cleanup. Reuse those existing boundaries rather than building another
staging service. This is a source observation; retention was not measured live.

- [ ] Complete eligible personal/test Business or Creator account discovery and selection through Meta.
- [ ] Issue a one-use pairing code privately and pair the test Mac; verify the ready state and expiry/reuse errors.
- [ ] Add best-effort deletion of objects owned by the publication attempt after the provider is finished fetching them, on both success and failure.
- [ ] Preserve the real publication result if cleanup fails; do not retry an already-published post because deletion failed. Retain TTL cleanup as fallback and redact diagnostics.
- [ ] Add focused tests for cleanup after success/failure, cleanup failure without duplicate posting, and expiry fallback.
- [ ] Publish an approved single static image. Record provider ID/visible result, staged-object removal, scheduled publication, and imported media/insights.
- [ ] Verify expired URLs become inaccessible and the deployed scheduled cleanup removes expired objects. R2 remains private.
- [ ] Add the existing DELETE endpoint to the Worker README when implementing/documenting the cleanup caller.

**Done when:** pairing, static-image publish/schedule/import, immediate cleanup,
fallback expiry, and reconnect/error behavior pass with a controlled account.

**Existing implementation:** [publication orchestration](../src-tauri/src/db/mod.rs),
[desktop staging client](../src-tauri/src/media_staging.rs),
[Worker](../workers/media-staging/src/index.js),
[staging runbook](../workers/media-staging/README.md).
**Regression IDs:** ACCT-03, SVC-03, POST-06/07/10, RPT-01.

### CP-07 — Accept Unsplash and Klipy workflows

- [ ] Unsplash: search in the packaged app, verify attribution, permitted download behavior, and the download trigger described by the current provider requirements.
- [ ] Klipy: search, preview, attach to a draft, and complete an approved compatible-provider publish attempt.
- [ ] Verify Klipy references remain references; temporary publish files are removed after success/failure and do not enter the reusable library, backups, or support exports.
- [ ] Preserve visible branding and content attribution, including the empty search state.
- [ ] Verify missing/invalid credentials, empty results, and rate-limited requests produce useful redacted errors.
- [ ] Recheck current content filters/blocklists, terms, and production-access requirements before broader use; record production approval separately from a test-key success.

**Done when:** both end-to-end media paths pass on the package, with required
attribution and storage behavior. Provider-supplied media rights remain distinct
from permissions to distribute the app. Keep manual/local GIF import available.

**References:** [Unsplash](PROVIDER_SETUP.md#unsplash), [Klipy](PROVIDER_SETUP.md#klipy),
[GIF policy](PROVIDER_SETUP.md#gif-content-policy). **Regression IDs:** MEDIA-05/06.

### CP-08 — Prove live scheduling, retry, and emergency recovery

- [ ] Schedule a controlled post with the app open, Mac awake, and network available; compare displayed timezone, stored execution instant, and provider result.
- [ ] Confirm app-close/sleep/offline behavior and recovery after reopening/reconnecting. Record actual timing; the MVP has no always-running cloud scheduler.
- [ ] Verify moving a scheduled post to draft or deleting it before execution cancels the queued publication.
- [ ] Exercise a safe credential failure and a rate-limit/deferred-job case in an appropriate test environment; avoid deliberately exhausting provider quotas.
- [ ] Verify retries and mixed-account partial success do not duplicate already-published provider posts. Retain provider IDs and job outcomes as evidence.
- [ ] Check stale-job/failed-import recovery through System's existing product controls, plus understandable notifications and errors.
- [ ] Follow the support runbook's emergency stop and controlled reconnection flow.

**Done when:** provider-visible timing and recovery behavior agree with local status,
with no duplicate publication or unexpected queued action. Use existing automated
DST/timezone/idempotency tests and add coverage only for newly discovered gaps.

**References:** [support](SUPPORT_RUNBOOK.md), [job architecture](ARCHITECTURE.md#background-work).
**Regression IDs:** POST-06/07/09/10, CAL-01/03/04, SYS-01/02.

### CP-09 — Finish accessibility and dense-data acceptance

- [ ] Run full spoken VoiceOver flows on the exact signed candidate: navigation, tabs, composer, provider steps, dialogs, nested emoji/media pickers, restore selection, progress, and errors.
- [ ] Verify accessible names, selected/current states, reading order, focus trap/return, Back/Escape, and error/progress announcements by listening and operating the controls.
- [ ] Check long account names, multiple providers, long labels, dense Post library rows, and the large calendar agenda.
- [ ] Exercise System/Light/Dark, keyboard-only use, 1100×720 minimum window, 1280px and wide widths, 200% zoom/reflow, and reset to 100%.
- [ ] Fix reproducible failures through the shared dialog, tab, feedback, and contextual-editor components; capture before/after evidence at the same conditions.

**Done when:** complete spoken workflows and dense-data layouts pass with a human
tester. Existing ARIA/source tests and the earlier limited VoiceOver session are
not this acceptance. Record accessibility failures by user-flow ID.

**References:** [cross-cutting accessibility](USER_FLOWS.md#cross-cutting-accessibility),
[dialog focus](../resources/desktop/src/dialogFocus.js),
[tab navigation](../resources/desktop/src/tabNavigation.js).

### CP-10 — Finish Local AI machine, offline, and quality acceptance

Keep Labs opt-in and the current native LiteRT/older-system Wasm architecture.
Use the [packaged acceptance procedure](LOCAL_AI.md#packaged-app-acceptance).

- [ ] Record actual runtime on another Apple Silicon Mac with macOS 14+ and on a supported older macOS configuration where the Wasm fallback applies.
- [ ] Confirm and document the minimum supported app OS from the package/configuration. The native library's macOS 14 minimum is a separate boundary from the app/fallback minimum.
- [ ] Measure representative transparent PNGs and photographs up to 512×512: runtime, dimensions, load/inference/total time, progress, typing/navigation responsiveness, cancellation, and retry.
- [ ] Check 4× output, original hashes, alpha preservation, derivative metadata, oversize rejection, helper cleanup, and absence of partial saved output after cancellation.
- [ ] Have the owner review faces, lettering, logos, texture, and tile boundaries at intended output size. Agree on acceptable results and limits; no performance target is invented here.
- [ ] Complete a whole-app offline test on an approved isolated test machine or a proven app-specific boundary. Establish that both fresh native and WebKit network controls fail while bundled inference and import/save succeed.
- [ ] Keep the native-helper offline result separate: its `full_app_offline_accepted: false` is intentional. Do not reuse previously failed whole-app sandbox/LuLu attempts as evidence.
- [ ] Exercise crop/preflight/property search and editable/discardable alt-text drafts, including edit-overwrite confirmation; turn Labs off after testing.

**Done when:** supported hardware/runtime paths, full-app offline processing, and
human output quality have explicit results. If a configuration is unsupported,
record that product decision and update the compatibility claim rather than
silently counting it as tested. Do not add cloud inference, a replacement model,
a loopback server, or weaker permissions just to bypass an acceptance failure.

**Regression IDs:** MEDIA-07, SET-02, SYS-04/05.

### CP-11 — Complete representative media, backup, and support acceptance

- [ ] Import the attached MP4 using the native picker; check playback, dimensions/duration, thumbnail, and System's bundled FFmpeg/FFprobe status. Also use a representative real-world video; a tiny static clip does not stress the whole video path.
- [ ] Check static-image/GIF intake, URL-download errors, missing/unsupported files, and missing or unresponsive media tools.
- [ ] Delete only an app-owned test copy and confirm the original user-selected file remains unchanged.
- [ ] Back up representative drafts, account-specific versions, labels, schedules/history, originals, AI derivatives, and metadata.
- [ ] Restore through the supported UI into an isolated workspace; check validation, safety backup, counts, hashes, and normal editing after restore.
- [ ] Verify controlled-account reconnection when Keychain secrets are unavailable and readiness preservation when they legitimately remain on the same Mac.
- [ ] Distinguish saved SQLite drafts from the device-local recovery buffer. Appearance and recovery storage are not included in the documented database/media backup.
- [ ] Verify redacted support information and logs with controlled secret-bearing operations; inspect privately without copying secret values into evidence.
- [ ] Check meaningful desktop notification outcomes and denied-permission behavior.

**Done when:** representative user-data recovery, media handling, and secret-free
support exports pass on the package. The earlier 30-file isolated restore remains
useful evidence, but did not contain production credentials.

**References:** [backup architecture](ARCHITECTURE.md#backup-and-restore),
[support runbook](SUPPORT_RUNBOOK.md). **Regression IDs:** MEDIA-01/02/03,
SET-01, SYS-01/03/04/05.

### CP-12 — Prove another user can complete setup

- [ ] Give a new test user the released build and existing guide in a clean workspace.
- [ ] Observe installation → provider setup → connection → first import/search → draft → approved publication where supported.
- [ ] Record time to first success, where they stall, misleading copy, missing prerequisites, and when they need developer help.
- [ ] Verify Instagram pairing requires no ordinary-user Cloudflare/Wrangler access or reusable operator token.
- [ ] Verify setup copies contain no secrets and guide users to the right account/portal context.
- [ ] Fix repeated friction in `providerSetup.js` and the existing shared controls; keep one catalog, one save flow, and one setup guide.
- [ ] Repeat the failed portions with the revised package/guide and retain results.

**Done when:** a new user reaches the intended first outcome using the documented
flow and can understand a recoverable setup failure without raw database/queue tools.

**References:** [five-step setup](PROVIDER_SETUP.md#the-five-step-path),
[provider catalog](../resources/desktop/src/providerSetup.js). **Regression IDs:**
SVC-01/02/03, ACCT-01/06, CORE-01, POST-01.

### CP-13 — Complete wider-distribution approvals and operational ownership

This work is conditional on onboarding people beyond controlled test identities.
Inspect current provider requirements and saved portal state at execution time.

- [ ] TikTok: complete the Production draft from the validated Sandbox contract, obtain approval to submit the reviewed packet, record submission, and then record the actual review decision. After approval, privately switch the intended broker/desktop client configuration and validate an account outside the Sandbox target list.
- [ ] Meta: verify required Advanced Access/app review and any current provider prerequisites for the intended unrelated users; verify a real user outside app roles can connect only after access is granted.
- [ ] Unsplash/Klipy: obtain and record production access appropriate to the real workflow and current terms.
- [ ] If organization use is intended, separately approve and onboard the named Dust Wave accounts after personal tests pass.
- [ ] Resolve public product/legal-page availability in a normal browser; verify their content describes the actual local data, broker/staging, and update-check behavior.
- [ ] Assign owners for provider apps/credentials, broker/D1, staging/R2 and cleanup, Apple/updater signing, releases, backups, and support/emergency revocation.
- [ ] Verify service recovery/deployment inputs privately, including the broker's intended credential mode and preservation of encryption keys/device pairings.
- [ ] Define recurring operational responsibilities for provider changes, service failures, retention cleanup, dependency advisories, and license/notices changes. Creating a monitor is a separate execution step if requested.
- [ ] Complete the existing [product-risk review](BEST_PRACTICES.md#release-review-checklist) and record decisions for any remaining red flags.

**Done when:** outside-user access is demonstrated for the advertised integrations,
review decisions are recorded, and every ongoing service has an owner and recovery
procedure. Saved portal values, healthy HTTP endpoints, and submitted reviews are
not approval or end-user acceptance.

### CP-14 — Validate and accept the completion release

Use [release operations](RELEASE_OPERATIONS.md) for the exact build, signing,
notarization, publication, and rollback procedure.

- [ ] Assemble the final changes and evidence; keep versions synchronized and write release notes describing actual final behavior and limits.
- [ ] Run the full applicable source/release checks and inspect any failures or advisory findings.
- [ ] Build and verify the exact signed/stapled app and DMG, updater archive/signature, and manifest. Repeat package-sensitive acceptance after any rebuild.
- [ ] Run the previous-public-version → candidate update/relaunch test with representative data, plus the clean-Mac and affected provider/UI/media checks.
- [ ] Present the concrete release and remaining decisions to the owner for go/no-go. Publish only when that action is authorized.
- [ ] After publication, independently verify the latest feed and public asset digests, signatures, notarization, installer layout, and real update result.
- [ ] Preserve current/rollback assets and accepted evidence; update this document, Changelog, and the release record.

**Done when:** all required MVP gates pass on the final version, any scope decisions
are explicit, and the owner accepts it. Wider-distribution completion also requires
CP-13. Keep 0.1.9 as the currently recorded rollback baseline until the release
owner deliberately changes it after accepting a newer baseline.

## Recommended execution order

1. **Repair and unblock:** CP-01 editor update, CP-02 font evidence, CP-06 staging
   cleanup. In parallel with available people, locate the second Mac/tester and
   inspect existing provider access. These are independent workstreams, not a
   requirement to create parallel agents or new tasks.
2. **Controlled-account proof:** CP-04 TikTok, CP-05 direct providers, CP-06 Instagram,
   CP-07 external media, and CP-08 scheduling/recovery. Prepare review packets from
   the successful results; provider review turnaround is external.
3. **Packaged acceptance:** CP-03 and CP-09 through CP-12 on the final candidate.
   Reuse earlier results only when their exact build, configuration, and tested
   boundary still apply.
4. **Accept and distribute:** CP-14 for the supported MVP; finish CP-13 before
   claiming readiness for unrelated users.

The critical external dependencies are a second suitable Mac/older-system test
environment, a human VoiceOver/output reviewer, font permission evidence, an eligible
Instagram professional account, and provider access/review decisions. Engineering
can prepare fixes and test procedures before those become available. A reliable
calendar estimate depends on assigning those resources and inspecting portal state.

## Evidence and completion tracking

Use [Provider Setup's acceptance record](PROVIDER_SETUP.md#acceptance-record) for
provider evidence. For other work, retain a concise record with:

| Field | Required detail |
| --- | --- |
| Identity | CP ID and existing user-flow IDs; tester and date |
| Build/environment | Commit, package version/hash, OS/chip, runtime, test or production mode |
| Preconditions | Controlled account/data scope; required approval reference where applicable |
| Action | Exact workflow/format/failure condition exercised |
| Result | Pass, fail, blocked, or not tested; actual output and expected outcome |
| Proof | Provider ID/visible result, redacted log, screenshot/recording, or verification report |
| Recovery | Cleanup, retry/reconnect, original/data preservation, duplicate-publication result |
| Follow-up | Remaining defect, owner, dependency, and next concrete action |

Keep secrets, credentials, private account data, and one-time codes out of records.
Large recordings, fixture databases, and generated reports belong in private local
evidence storage or ignored `artifacts/`; commit only safe durable conclusions and
links. Evidence from prior sessions should remain dated and labeled as recorded.

### Final sign-off checklist

- [ ] CP-01 and CP-02: dependency and redistribution work resolved.
- [ ] CP-03: clean install and existing-data update accepted on another Mac.
- [ ] CP-04 through CP-07: every intended integration and advertised media format has a result.
- [ ] CP-08: schedule/retry/recovery behavior accepted without duplicates.
- [ ] CP-09: spoken accessibility and dense layouts accepted.
- [ ] CP-10: supported AI runtimes, full-app offline behavior, and output quality accepted.
- [ ] CP-11: representative media, backup/restore, notifications, and redaction accepted.
- [ ] CP-12: another user completed onboarding.
- [ ] CP-14: final signed release and update path verified; owner recorded go/no-go.
- [ ] CP-13, when wider distribution is intended: required approvals and ongoing ownership complete.

### Verification commands

For documentation-only edits:

```sh
npm run docs:check
git diff --check
```

For the eventual implementation/release candidate, use the existing release suite
and run extra checks according to the changed boundary:

```sh
npm audit --omit=dev
npm run desktop:release:check
npm run docs:check
npm run local-ai:models:check
npm run mvp:release:notes
npm run mvp:release:notes:check
```

The release suite already includes the desktop build, UI/Rust checks, packaging
contracts, and both Worker suites. Follow the release runbook for signed artifacts
and updater acceptance; the commands above do not publish anything. If the shared
editor packages change, also verify the retained Mixpost asset build and any
affected legacy tests. Do not mistake a readiness script's missing local build
outputs or environment credentials for a failed public release or missing Keychain
credentials. Some manual gates in those scripts are deliberately always manual.

## Scope to keep stable

Preserve the local-first Tauri/Rust/Vue/SQLite architecture and Keychain secret
boundary. Labs stays opt-in, with the existing 512px input and 2048px output
limits, native LiteRT path, and older-system fallback. Keep local tests, CI,
public release, installed-app behavior, live providers, and physical-machine
acceptance as separate claims.

The existing scope leaves these for separate product decisions:

- TikTok direct API publishing and its additional approval scopes.
- Instagram video, reels, stories, carousels, and GIF publishing.
- Facebook Groups and publishing to personal Facebook timelines.
- Intel/universal distribution.
- Cloud sync, team collaboration, hosted telemetry, or remote AI inference.
- Embedding-based search, model-generated image captions, larger AI input limits,
  and additional experimental runtime/model engines.
- A new app-lock system or desktop routing architecture.
- Removal of the retained Mixpost package.

Do not expand those features to compensate for missing evidence in the supported
MVP. Consult [Features](FEATURES.md#deferred-work) before changing scope.

## Documentation and maintenance

This file owns the status summary and completion checklist. Update evidence here
when a CP item closes, including date, tested version, and remaining limitations.
Do not maintain a second status document. Generated checkout reports remain under
ignored `artifacts/`; link to each specialist guide instead of duplicating it.

### Sources reconciled

The supplied `SOCIAL_COMPLETION_PLAN.md` matched the repository completion plan.
The supplied `Social - PROJECT_STATUS.md` was the older September 6 snapshot. The
consolidation keeps the newer located-media inventory, September 15 audit results,
Instagram cleanup finding, and current release-guide links, while preserving the
older unique UX, recovery, integration, and cleanup evidence in this document.

The audit read all 17 maintained Markdown files: the repository README, license,
security policy, all 11 guides in `docs/`, both Worker READMEs, and the media-sidecar
README. It also reviewed the package scripts/lockfile, Tauri configuration, selected
provider/editor/media source, release/readiness scripts, focused tests, recent Git
history, GitHub release/workflow/open-work metadata, and the attached files. Earlier
August 29 context was used to identify the asset's original purpose and checksum;
current docs/source supersede its older setup and document-location details.

| Authority | Use |
| --- | --- |
| [README](../README.md), [Architecture](ARCHITECTURE.md), [Features](FEATURES.md) | Product scope, data ownership, implementation boundaries |
| This document; [User flows](USER_FLOWS.md) | Current state, completion checklist, and regression IDs |
| [Provider Setup](PROVIDER_SETUP.md), [TikTok broker](../workers/tiktok-broker/README.md), [media staging](../workers/media-staging/README.md) | Provider onboarding, service boundaries, acceptance records |
| [Local AI](LOCAL_AI.md), [Support](SUPPORT_RUNBOOK.md) | Offline/hardware/quality acceptance and recovery procedures |
| [Release operations](RELEASE_OPERATIONS.md), [Changelog](CHANGELOG.md) | Published release evidence and version history |
| [Security](../SECURITY.md), [Best practices](BEST_PRACTICES.md), [Third-party notices](THIRD_PARTY_NOTICES.md), [License](../LICENSE.md) | Existing security, product-risk, and redistribution gates |

### Historical cleanup evidence

The September 6 cleanup removed the merged release branch and disposable release
worktree and moved twenty obsolete build/test entries (16.84 GiB logical) into a
dated, recoverable Trash folder. Installed apps, app data, Keychain, signing
material, dependencies, and runtime downloads were preserved. Its documentation,
version-alignment, and report checks passed; no app code or release changed in
that cleanup. Historical records described local 0.1.10/0.1.9 release caches, but
the standard `~/Library/Caches/DustWaveSocial` directory is absent in the September
15 inventory. GitHub Releases remain the distribution source; do not claim an
older local cache is still available without checking.

### September 15 consolidation and retention inventory

The requested publication is the consolidated documentation on GitHub. No app
version change, provider deployment, or unfinished completion-plan task is part of
this documentation update.

The pre-publication inventory found one checkout, only local/remote `main`, no
open pull requests, and no local target tree, generated frontend build, fixture
artifact directory, or standard Social cache to remove. Retain installed npm
dependencies, the prepared FFmpeg/FFprobe and native LiteRT runtime/helper with
their receipts, bundled model assets, and private local Worker configuration for
development and testing. Preserve app data, credentials, signing material, source,
tests, and release tags. Move superseded document copies to dated Trash with a
restore record after publication; keep one updated Desktop copy of this guide.

Run `npm run docs:check` and `git diff --check` after documentation changes. The
publication commit and CI outcome are verified separately from desktop-release
or provider acceptance. This consolidation does not close any unchecked CP item.
