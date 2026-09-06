# Social: current state and next steps

Updated: 2026-09-06. Audience: the project owner, engineering, and acceptance testers.

This is the authoritative current-state and priority list. It separates shipped
capabilities from demonstrated outcomes. Procedures live in the linked guides;
do not copy their provider values, release commands, or test matrices here.

## Where the project stands

[Social 0.1.10](https://github.com/aindaco1/social/releases/tag/v0.1.10) is the
official Apple Silicon macOS release. The live updater feed, five public assets,
signing/notarization, and automatic 0.1.9 → 0.1.10 update/relaunch were verified.
The [release record](RELEASE_OPERATIONS.md#published-0110-evidence) holds exact build,
checksum, and notarization evidence. Version 0.1.9 is the rollback baseline.

The final read-only installed-app inventory reports 0.1.10. This cleanup did not
install or launch it; that version observation is not a fresh updater or
user-data acceptance result.

The foundation is implemented; broad user and provider acceptance is not complete.
The current focus is validating the released app on other Macs and closing the
personal-account integration matrix—not adding more experimental engines or
redesigning the platform.

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
evidence, not a fresh credential or live publishing check. No portals, app data,
Keychain values, or live accounts were changed during this cleanup.

| Integration | Evidence / present boundary | Next concrete check |
| --- | --- | --- |
| X | Adapter and stored-account evidence; API tier governs allowed operations | Refresh the personal account; verify a controlled post, scheduled post, import, and retry without duplicates |
| Facebook Page | Shared Meta setup and test-Page record | Verify the login configuration and minimal required permissions; refresh/import, then explicitly approve a test Page post |
| Instagram | Professional-account and one-use staging paths implemented; no connected account in the last UI review | Select an eligible personal/test professional account, pair the Mac, publish one static image, verify deletion of staged media and insights import |
| Mastodon | Per-server registration/OAuth and stored-account evidence | Refresh/import and run a controlled media post plus expected-error recovery on the selected server |
| TikTok | Broker and Assisted mode implemented; stored-account evidence does not prove current Sandbox scopes or Production approval | Revalidate the Sandbox authorization/import, produce a real authorization-and-analytics review demo, then assess Production review |
| Unsplash | Account creation was reported; demo/public access-key setup is documented | Verify search, attribution, and download-trigger behavior in the released app |
| Klipy | The August 28 signed local test returned 18 results with branding | Recheck test-key search/attach flow; verify content filters and production access before broader use |

Use [Provider Setup](PROVIDER_SETUP.md) for exact portal values and the single
acceptance-record format. The small local TikTok provenance/QA packet remains,
but its listed MP4 is absent from this checkout and a static test post is not an
end-to-end app-review demo. Locate the original or make a fresh authorized demo;
do not infer a public post or review submission from the packet.

The cleanup's read-only preflight received healthy responses from both Worker
health endpoints. It also reported six configuration warnings: five desktop
provider environment checks (which do not inspect saved Keychain credentials),
and absent `TIKTOK_CLIENT_KEY` / `TIKTOK_CLIENT_SECRET` in the checked environment
and GitHub secret-name inventory. This does not prove the live Worker lacks its
secrets. Before a broker redeploy, reconcile the intended Sandbox/Production
values through the existing private provisioning path without printing them.

## Prioritized next steps

| Priority | Work and owner | Definition of done |
| --- | --- | --- |
| P0 | Owner + tester: validate 0.1.10 on another Apple Silicon Mac | Complete the [other-Mac checklist](RELEASE_OPERATIONS.md#testing-on-another-mac), recording OS/chip/version, update/relaunch, data/credential survival, actual AI runtime, timings, cancel/retry, and output review; each result is pass/fail/not tested |
| P0 | Engineering: address the known editor dependency advisory | Upgrade to a currently patched compatible Tiptap release; keep the existing editor/schema seam; pass paste/link/emoji/draft/undo/security regression tests and a rendered-app check; rerun the production audit without calling a boundary mitigation a library fix |
| P0 | Owner: close the recorded Gambado font-distribution gate | Record redistribution permission for the bundled fonts, or authorize replacement through the existing font tokens and perform visual QA; do not assume possession of font files establishes permission |
| P1 | Owner + engineering: complete the personal-account matrix above | Each intended integration has a dated provider-visible result, expected error/reconnect behavior, and no duplicate publication or secret leakage; request approval before externally visible test actions |
| P1 | Tester: finish offline and accessibility acceptance | On an approved isolated test machine, both native and WebKit uncached controls fail while bundled upscaling succeeds; separately complete spoken VoiceOver dialogs, pickers, progress/errors, and dense data in both themes |
| P1 | Engineering + a new test user: prove setup is understandable | Follow the existing guide from a clean workspace, record stalls/time-to-first-success, and fix repeated friction in the shared catalog/components rather than creating a second onboarding path |
| P2 | Owner: prepare wider distribution | Recheck Meta/TikTok review state, Unsplash/Klipy production access, ownership/support/backup procedures, and all remaining [product-risk gates](BEST_PRACTICES.md); make an explicit go/no-go decision |

The [security policy](../SECURITY.md#0110-dependency-review) records the current
Tiptap 2.x advisory and tested input boundary. The library itself remains
unpatched in 0.1.10. Provider terms, review requirements, and dependency advisories
must be rechecked when their work begins; this document is not a live portal audit.

## Scope to keep stable

- Labs stays opt-in/off by default. Current static-image input limit is
  512×512px, output 2048×2048px; preserve originals and require review.
- Continue the native LiteRT path and existing older-system fallback. Do not
  introduce a loopback server, cloud inference, model replacement, or weakened CSP
  to bypass acceptance.
- TikTok direct API publishing, richer Instagram formats, Facebook Groups,
  Intel/universal builds, cloud/team features, embedding search, and model-generated
  captions remain deferred product decisions.
- Keep the retained Mixpost package until its removal is separately decided;
  deleting build outputs is not authorization to remove a supported code path.
- Keep local verification, CI, public release, installed-app behavior, live-provider
  results, and physical-machine acceptance as separate claims.

## Documentation and maintenance

| Question | Single owner |
| --- | --- |
| What is done and what is next? | This file |
| What can the product do? | [Features](FEATURES.md) |
| How do users reach each outcome / which regression ID applies? | [User flows](USER_FLOWS.md) |
| How is it built? | [Architecture](ARCHITECTURE.md) |
| How do providers and GIF policy work? | [Provider Setup](PROVIDER_SETUP.md) |
| How do local-AI use, safety, and acceptance work? | [Local AI](LOCAL_AI.md) |
| How do we build, verify, publish, test updates, and roll back? | [Release operations](RELEASE_OPERATIONS.md) |
| What changed in each release? | [Changelog](CHANGELOG.md) |
| What third-party software and attribution do we distribute? | [Third-party notices](THIRD_PARTY_NOTICES.md) |
| How do we troubleshoot safely? | [Support](SUPPORT_RUNBOOK.md) |

The chronological UX experiment log and standalone GIF decision were consolidated;
their committed history remains in Git. Generated local build/readiness snapshots
now belong under ignored `artifacts/`, not inside this status file or the release
runbook. Run `npm run docs:check` after documentation changes.

Cleanup on 2026-09-06 removed the merged `release/v0.1.10` branch locally and
remotely and deregistered the disposable release worktree. Twenty obsolete
build/test entries (16.84 GiB logical usage) went to a dated, recoverable macOS Trash
folder with a restore record; Trash was not emptied. Current/rollback public
assets and release validation reports are retained under
`~/Library/Caches/DustWaveSocial/releases/v0.1.10` and `v0.1.9`.
Installed apps, app data, Keychain, signing material, dependencies, and the pinned
runtime download cache were preserved. Rebuild outputs only when needed; do not
restore deleted experimental plans as the active roadmap.

Cleanup validation: 160 JavaScript tests, the 17-document link/catalog check,
13 local heading links, release-version alignment, and report generation/check
passed. The preflight warnings above are retained, not hidden. No Rust/app code
changed, no full native rebuild was run after cleanup, and no new release was made.
