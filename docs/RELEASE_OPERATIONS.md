# Dust Wave Social release operations

Updated: 2026-09-06. Audience: release maintainers and acceptance testers.

This runbook owns release procedures and immutable publication evidence.
[Project status](PROJECT_STATUS.md) owns current gaps, priorities, and the go/no-go
worklist. [Provider Setup](PROVIDER_SETUP.md) owns exact integration values.
A public engineering release does not, by itself, close every product acceptance
gate.

## Published 0.1.10 evidence

[Social v0.1.10](https://github.com/aindaco1/social/releases/tag/v0.1.10) was
published on 2026-09-06 from commit
`d37821a53c84a1f12be18da32ce9dcf5041d8b03`, retaining `com.dustwave.social`
and the existing updater key. Version 0.1.9 is the rollback baseline.

- [Post-merge main CI](https://github.com/aindaco1/social/actions/runs/34057979708) passed before tagging.
- The [tag workflow](https://github.com/aindaco1/social/actions/runs/34058509458) passed in approximately 12 minutes: 156 JavaScript tests, 150 Rust tests, two six-test Worker suites, signed packaging, notarization/stapling, mounted-DMG verification, packaged launch, native-helper offline inference, and public updater verification.
- Apple accepted app submission `55771d73-1445-4525-b1bd-7bb15be6f5d9`.
- Apple accepted DMG submission `99036d47-10a7-4904-91ab-460a9048a7cb`. Both artifacts passed Gatekeeper and strict signatures.
- The public updater installed 62,246,193 signed bytes into a staged 0.1.9 app and relaunched as 0.1.10, PID 38059 → 38096.
- Independent downloads matched all five GitHub digests, the live latest feed, notes, and signature asset. DMG/ZIP/updater apps contained the same 28 regular files with valid identity, arm64 architecture, signatures, and staples.
- The downloaded native helper (`e84d7a28e0c398192e1fd82e67c24a28d5ba8bd3c3d030d8cf2d0840b525be7f`) repeated reference-identical one/four-thread inference, TCP/UDP IPv4/IPv6 denial controls, cancellation/reaping, and malformed-frame rejection. This is native-only evidence, not full-app offline acceptance.
- All five 0.1.9 rollback assets were downloaded and checksum-verified; its mounted DMG passed trust checks. The installed 0.1.9 app was not replaced or launched by release validation.

| Published asset | SHA-256 |
| --- | --- |
| `Dust.Wave.Social_0.1.10_aarch64.dmg` | `5e9ec2c368ef1cce4a8c5de3a2353b668b869a972b2df28eb7529b206a66f8a9` |
| `Dust.Wave.Social.app.tar.gz` | `ce6bafd5ced108a1f5c059a33d00ca192953e1397e68784b9cedd4cf1e8e9f9c` |
| `Dust.Wave.Social.app.tar.gz.sig` | `5ab464867d227448c50773f375f13d6350eb33748f97f1b98dc01109057036f8` |
| `Dust.Wave.Social.app.zip` | `c13e8ce1dc3a37b3663b11bf548f2fe889b0bd2259057dff1432b94c08021dbe` |
| `latest.json` | `eeb918169ed1ccf6ee69d74a82838dae758734f8109678c98149bbedd93b9712` |

Release notes and in-app updater notes are extracted from one versioned entry in
[Changelog](CHANGELOG.md). Older release evidence remains in the
[versioned repository history](https://github.com/aindaco1/social/tree/v0.1.10/docs)
and each GitHub Release; do not carry obsolete candidate logs into the active plan.

## Build and verify

Start with a clean, reviewed source tree and preserve current/rollback installers.
Local release automation reads existing Apple/updater material from the sibling
`Apple Auth` folder or environment; never log or commit its values.

```sh
npm ci
npm run desktop:release:preflight
npm run mvp:launch:readiness
npm run desktop:release:build:notarized:with-media-and-updater
npm run desktop:macos:notarize
npm run desktop:release:artifact-check -- --require-updater --require-stapled
npm run mvp:release:notes
npm run mvp:release:notes:check
```

The build wrapper runs `desktop:release:check` once. Media must be the reviewed
LGPL-only sidecars built with `npm run desktop:media:build-lgpl`; keep the source,
hashes, build flags, and licenses in [Third-party notices](THIRD_PARTY_NOTICES.md).
Do not substitute arbitrary Homebrew binaries or GPL/nonfree builds.

Generated readiness output is `artifacts/release-readiness.md`, deliberately
ignored by Git. It describes the current checkout, not public-release availability.
After cleanup it is expected to report missing local build outputs; this does not
invalidate the published release. Regenerate it after building rather than
committing transient counts or machine paths.

Expected outputs under `src-tauri/target/release/bundle/` are the stapled app,
stapled DMG with an /Applications link, updater tarball/signature, and `latest.json`.
Do not clean or move them during signing, notarization, or acceptance.

The macOS wrapper normally uses `~/Library/Caches/DustWaveSocial/target` through
`src-tauri/target`; `DUSTWAVE_RELEASE_USE_PROJECT_TARGET=true` keeps the project's
own target. Verify that an existing target path is safe and writable before the
wrapper runs. For packaging outside iCloud, use a separate clean checkout; do not
replace an active target or copy unrelated user changes.

## Publish and verify the public channel

1. Synchronize package/Cargo/Tauri versions and add a changelog entry.
2. Run source checks and mounted signed/stapled-DMG validation.
3. Merge reviewed work; wait for checks on the exact main commit to pass before tagging.
4. With explicit release approval, push the matching version tag. The existing Desktop workflow reruns gates, signs/notarizes, publishes the five assets, verifies the public manifest, and stages an update from the previous public version.
5. Confirm the workflow passed and the release is public, non-draft, non-prerelease, and selected by the latest endpoint. The workflow returns a failed post-publication candidate to draft.
6. Independently download all five assets; compare hashes, signature/manifest version, code identity, notarization, and installer layout. Record the actual prior/new version and process-hop result.
7. Preserve the accepted public assets and previous known-good installer; update this release record and [Project status](PROJECT_STATUS.md).

The workflow restores only exact-commit Cargo intermediates, never final binaries.
Updater discovery is quiet at launch; downloading, installation, and restart still
require the user's explicit action. Keep the signing key stable. Do not generate a
replacement updater key for a routine release.

## Testing on another Mac

Use an Apple Silicon Mac and the official build. Record model/chip, macOS version,
starting Social version, and whether it contains existing data.

1. Back up representative data from System and preserve the previous DMG.
2. From 0.1.9, use the top-right Update action or System controls to install 0.1.10. Verify automatic relaunch and the displayed version.
3. Verify drafts, media, settings, provider setup, and Keychain-backed readiness survived; reconnect only when actually required.
4. On a separate clean macOS account/Mac, drag the stapled DMG app into /Applications, launch without a trust warning, then quit/relaunch. Do not delete the operator's data to manufacture a clean test.
5. Follow [Using image upscaling](LOCAL_AI.md#using-image-upscaling). Record actual runtime, source/output dimensions, time, cancellation/retry, original preservation, and human output review for a transparent PNG and a photograph.
6. Complete keyboard/VoiceOver and dense-data checks against [User flows](USER_FLOWS.md). Test both themes at 1100×720, 1280px, and wide widths; 1024px is a CSS breakpoint, not the supported native minimum.
7. Use the separate [full-app offline procedure](LOCAL_AI.md#packaged-app-acceptance), with consent and explicit native/WebKit network controls. Never disconnect other apps' networking for this check without approval.
8. Record each check as pass/fail/not tested, with exact errors and redacted support output.

Versions 0.1.0–0.1.2 need one manual installation of a 0.1.3-or-newer DMG to fix their
updater resource bug. Updating from 0.1.4 needs one manual quit/reopen after install;
later versions implement automatic restart. Intel/universal acceptance is outside
the current distribution scope.

## Isolated unpublished UX candidates

Use a separate checkout/target and a locally created Tauri overlay with a distinct
product name, bundle identifier, and prerelease version. Set
`bundle.createUpdaterArtifacts` to false and updater endpoints/public key to empty.
The running identifier owns both app data and Keychain namespace. Omit provider
environment credentials; never copy production app data or use the production-ID
development signing runner.

Reuse `node scripts/build-macos-release.mjs --no-temp-keychain --media --bundles app --config <overlay-path>`,
with `DUSTWAVE_RELEASE_USE_PROJECT_TARGET=true`, `DUSTWAVE_SKIP_ADHOC_SIGN=true`,
and a distinct `CARGO_TARGET_DIR`. Notarize the explicit app path with
`node scripts/notarize-macos-app.mjs --app <app-path>`. Inspect the exact identifier,
version, arm64 architecture, deep/strict signature, staple, and Gatekeeper result.
Launch only the isolated app; verify empty accounts/queue and its data path.

An earlier staple never covers a rebuild. A ZIP of the stapled app is enough for
unpublished testing; it is not public-updater or production-data acceptance.
If notarytool crashes before confirming a submission, the shared wrapper supports
a bounded `--no-s3-acceleration` retry. Do not bypass signing or Gatekeeper.

## Rollback Plan

1. Stop affected scheduled publishing by quitting Social; preserve redacted support logs.
2. Back up app data before uninstalling or downgrading.
3. With release-owner approval, make a bad release undiscoverable by returning it to draft and verify the latest feed resolves to the known-good release.
4. Install the preserved 0.1.9 DMG on affected Macs; confirm Gatekeeper, launch, and data loading. The updater is not an automatic downgrade mechanism.
5. If data shape changed, restore a known-good Social backup through the supported restore flow, not ad-hoc SQLite edits. Backups exclude Keychain secrets.
6. Revoke/reconnect provider credentials only when indicated by the incident, and verify draft/media/account behavior before resuming jobs.
7. Record impact, owner, mitigation, and the explicit ship/no-ship decision.

Detailed recovery and redaction procedures belong in [Support](SUPPORT_RUNBOOK.md).
Provider and product acceptance belong in [Project status](PROJECT_STATUS.md), not
a generated local artifact inventory.

## Retention and repository hygiene

Keep one copy of the latest public release and one rollback version under
`~/Library/Caches/DustWaveSocial/releases/<version>`, with small validation reports.
GitHub Releases remain the published distribution source; local caches are
recoverable convenience copies, not the only backup.

Inventory exact paths and active processes/worktrees before cleanup. Move obsolete
builds/test evidence to a dated Trash folder with restore mappings; never empty
Trash, delete app data, remove signing material, or change firewall rules as part
of routine artifact cleanup. Keep dependencies and checksum-pinned runtime download
caches unless a separate dependency-cache cleanup is requested.

Delete only merged branches with no open PR/active worktree, preserving their
commit IDs and release tags. Keep generated overlays, secrets, credentials,
fixture databases, build outputs, and screenshots out of source control.
