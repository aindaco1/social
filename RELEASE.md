# Dust Wave Social Release Checklist

This checklist covers the Tauri/Rust desktop app. It keeps unsigned local builds, signed macOS distribution, notarization, and updater work separate so release steps stay reversible.

References verified on 2026-06-25:

- Tauri v2 macOS signing: https://v2.tauri.app/distribute/sign/macos/
- Tauri v2 environment variables: https://v2.tauri.app/reference/environment-variables/
- Tauri v2 updater: https://v2.tauri.app/plugin/updater/
- Dust Wave product and ethical risk review: `docs/BEST_PRACTICES.md`

## Local Preflight

Run these before cutting a desktop build:

```sh
npm ci
npm run desktop:release:preflight
npm run desktop:release:check
```

`desktop:release:preflight` reports local release inputs such as media sidecars, updater keys, Apple signing variables, provider credential environment variables, and GitHub/Cloudflare/Stripe CLI availability without printing secrets. `desktop:release:check` builds the desktop Vue bundle, verifies Rust formatting, and runs the Rust test suite. Release readiness is handled by this checklist and CI, not by a production app panel.

Before signing or publishing artifacts, complete the release review checklist in `docs/BEST_PRACTICES.md`. Record any ethical red flags, abuse risks, privacy risks, or operator-confusion risks in the release notes or issue tracker with an owner and ship/no-ship decision.

## CI Preflight

`.github/workflows/desktop.yml` runs `npm run desktop:release:check` on pushes to `main`/`master` and on pull requests.

The same workflow has a manual `workflow_dispatch` path. Set `build_bundle` to `true` to build and upload macOS `.app.zip` and `.dmg` artifacts from GitHub Actions. Optional inputs can request staged media sidecars and signed updater artifacts. Set `publish_release` to `true` with a `release_tag` to create a draft GitHub Release from those artifacts.

Updater artifacts require `TAURI_SIGNING_PRIVATE_KEY`, optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, and `TAURI_UPDATER_PUBLIC_KEY` in repository secrets or variables. The workflow now errors if updater artifacts are requested but no updater files are generated.

## Apple Auth Folder

Local signing automation can read release credentials from the sibling iCloud folder `Apple Auth` without committing private material. The supported local files are:

- `developer-id-application.p12`: Developer ID Application certificate.
- `apple-p12-password.txt`: password for the `.p12`.
- `AuthKey_<key-id>.p8`: App Store Connect API key file for notarization.
- `apple-api-issuer.txt`: App Store Connect issuer UUID. This is required for the current notarization script.
- `tauri-updater-private.key`, `tauri-updater-password.txt`, and `tauri-updater-public-key.txt`: generated Tauri updater signing material.

Generate updater signing material once with:

```sh
npm run desktop:updater:keys
```

The updater private key and password stay outside the repository. The public key is safe to copy into release configuration, but keep the generated private key backed up; installed apps cannot accept future updates signed by a replacement key.

## Unsigned Local Build

```sh
npm run desktop:build
```

Expected macOS artifacts:

- `src-tauri/target/release/bundle/macos/Dust Wave Social.app`
- `src-tauri/target/release/bundle/dmg/Dust Wave Social_0.1.0_aarch64.dmg`

Unsigned builds are useful for local verification only. Downloaded unsigned builds can trigger macOS trust warnings.

MVP builds target Apple Silicon macOS only. Intel and universal macOS artifacts are out of scope unless Dust Wave later changes distribution targets.

Local macOS builds are ad-hoc signed after bundling when no Apple signing variables are present. This does not replace Developer ID signing or notarization, but it makes local `.app` smoke tests and code-signature verification behave consistently:

```sh
npm run desktop:adhoc-sign
codesign --verify --deep --strict --verbose=2 "src-tauri/target/release/bundle/macos/Dust Wave Social.app"
npm run desktop:smoke:launch
```

`desktop:smoke:launch` runs the packaged `.app` executable with a temporary home directory and fails if the app exits before the smoke window completes.

After local verification, remove reproducible desktop build output without touching `node_modules`, source files, docs, or staged FFmpeg/FFprobe sidecars:

```sh
npm run desktop:clean
```

## Updater-Capable Build

After `npm run desktop:updater:keys`, set the final GitHub Releases repository slug:

```sh
export DUSTWAVE_RELEASE_REPO=aindaco1/social
npm run desktop:release:build:signed:with-media-and-updater
```

For unsigned local updater-config testing only:

```sh
export DUSTWAVE_RELEASE_REPO=aindaco1/social
npm run desktop:release:build:with-updater
```

`scripts/prepare-updater-config.mjs` writes `src-tauri/tauri.updater.generated.conf.json`, which is ignored by git. The file contains only the updater public key and GitHub Releases endpoint; the private signing key stays in the release environment.

Updater builds also generate `src-tauri/target/release/bundle/latest.json` from the signed `.app.tar.gz` and `.sig` files. That static JSON follows Tauri's required `version`, `platforms.<target>.url`, and `platforms.<target>.signature` shape and is uploaded beside the updater archive in GitHub Releases.

To configure a GitHub repository once the final release repo exists, preview and then apply the secret upload:

```sh
npm run desktop:github:release-secrets:plan -- --repo aindaco1/social
node scripts/configure-github-release-secrets.mjs --repo aindaco1/social --apply
```

The workflow expects the API key content in `APPLE_API_KEY_P8`; it writes a temporary `.p8` file on the GitHub runner instead of requiring a repository path secret.

## Product Risk Gate

Run this gate for every public release and for any internal release that changes publishing, automation, account connection, media import, backup/restore, reporting, notifications, support exports, or provider behavior:

- Review the release against `docs/BEST_PRACTICES.md`.
- Verify the app still redacts tokens, refresh tokens, client secrets, API keys, and sensitive credential material from logs, setup packets, onboarding packets, support exports, backups, and screenshots.
- Confirm externally visible actions retain clear operator intent, especially publish now, schedule, duplicate, retry, account selection, restore backup, disconnect account, and bulk delete.
- Red-team the changed workflows for misinformation, impersonation, spam, harassment, doxxing, account takeover, and accidental posting from the wrong account.
- Confirm desktop notifications are meaningful operational interruptions and not engagement-maximizing prompts.
- Confirm the release notes mention any new data collection, retention, export, backup, provider, or automation behavior.
- Do not ship until every red flag has an explicit mitigation or an accepted, documented residual risk.

## Optional Bundled FFmpeg And FFprobe

Dust Wave Social can package `ffmpeg` and `ffprobe` as Tauri sidecars so target Macs do not need Homebrew or a separate FFmpeg install.

First stage approved portable binaries for the current Rust target:

```sh
DUSTWAVE_FFMPEG_BINARY=/absolute/path/to/ffmpeg \
DUSTWAVE_FFPROBE_BINARY=/absolute/path/to/ffprobe \
npm run desktop:media:prepare
```

Then build with the sidecar config overlay:

```sh
npm run desktop:release:build:with-media
```

The sidecar build uses `src-tauri/tauri.media-sidecars.conf.json`, which merges Tauri `bundle.externalBin` entries for `binaries/ffmpeg` and `binaries/ffprobe`. The staging script writes target-triple filenames such as `src-tauri/binaries/ffmpeg-aarch64-apple-darwin`, matching Tauri's sidecar requirement.

Do not ship arbitrary Homebrew binaries as release sidecars. They are usually dynamically linked to `/opt/homebrew` libraries, commonly include GPL codec flags, and can still fail on clean Macs. Dust Wave's policy is LGPL-only FFmpeg/FFprobe sidecars. The staging script rejects binaries that report GPL licensing, `--enable-gpl`, `--enable-nonfree`, or common GPL codec flags.

For every release with media sidecars, fill in the FFmpeg/FFprobe section in `THIRD_PARTY_NOTICES.md` and archive the matching source, binary source, version output, license output, and configure line.

## macOS Signing And Notarization

For public macOS distribution outside the App Store, use a Developer ID Application certificate. The Tauri v2 bundler can read signing and notarization settings from environment variables.

Common CI/local variables:

- `APPLE_CERTIFICATE`: base64 encoded `.p12` signing certificate.
- `APPLE_CERTIFICATE_PASSWORD`: password used when exporting the `.p12`.
- `APPLE_SIGNING_IDENTITY`: optional explicit signing identity.
- `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`: Apple ID notarization path.
- `APPLE_API_KEY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_PATH`: App Store Connect API key notarization path.
- `APPLE_PROVIDER_SHORT_NAME`: only when the Apple ID maps to multiple teams.

Do not commit certificates, private keys, app-specific passwords, or API key files. Store them in the local keychain or CI secret storage.

On the local release machine, prefer the Apple Auth wrapper:

```sh
npm run desktop:release:build:signed:with-media
```

This creates a temporary signing keychain, signs the sidecars and app with the Developer ID Application identity, enables hardened runtime, creates a signed DMG with the app and an `/Applications` symlink, and restores the original keychain search list when it exits. The wrapper uses a direct `hdiutil` DMG path instead of Tauri's local DMG helper.

After adding the App Store Connect issuer UUID to `Apple Auth/apple-api-issuer.txt`, submit and staple the signed DMG:

```sh
npm run desktop:macos:notarize
```

Expected failure before the issuer file exists: `Missing Apple API issuer UUID`. Expected Gatekeeper result before notarization: `Unnotarized Developer ID`.

If Apple accepts the upload but the local wait times out while the submission is still `In Progress`, resume the same submission without uploading a duplicate:

```sh
npm run desktop:macos:notarize:wait -- <submission-id>
```

When Apple returns `Accepted`, the script staples the DMG and runs `spctl`.

To check the current status without waiting or stapling:

```sh
npm run desktop:macos:notarize:status -- <submission-id>
```

## Future Updater Gate

The Rust and JavaScript updater dependencies are installed, the desktop updater plugin is registered, the System screen can check/install updates when a build is configured, and `src-tauri/tauri.updater.example.json` documents the GitHub Releases endpoint shape. Do not merge permanent updater values into the live `tauri.conf.json` until these are ready:

- The final GitHub repository slug for release hosting.
- A release endpoint that serves the Tauri updater JSON over HTTPS, expected to be `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`.
- A generated updater keypair stored outside the repository.
- `TAURI_SIGNING_PRIVATE_KEY` and optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` available only in the release environment.
- The generated updater public key replacing `REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY` in the example config.
- `bundle.createUpdaterArtifacts` and `plugins.updater` enabled for release builds.
- `latest.json` uploaded to each GitHub Release beside the signed updater artifacts.

Tauri update signatures cannot be disabled. Losing the updater private key means already-installed apps cannot accept future updates signed by a replacement key.

## Desktop Permissions

The default Tauri capability intentionally grants only the plugin commands used by the desktop UI:

- `dialog:allow-open` for local media file selection.
- `notification:default` for native desktop health and publishing notifications.
- `opener:allow-open-url` plus `opener:allow-default-urls` for OAuth browser handoff.
- `updater:default` for signed update check, download, and install from the System screen.

Do not broaden these to plugin defaults unless a new UI workflow needs the extra command surface and the migration plan documents the reason.
