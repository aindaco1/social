# Dust Wave Social

Dust Wave Social is a local-first social publishing and reporting app for Apple Silicon macOS. It is built with Tauri, Rust, Vue, and SQLite, and keeps provider credentials in the macOS Keychain instead of the local database.

The repository also retains the original Mixpost Lite Laravel package while the desktop replacement completes live-provider and release acceptance. The desktop app does not require the Laravel server for normal operation.

## Current status

[Social 0.1.10](https://github.com/aindaco1/social/releases/tag/v0.1.10) is the
official Apple Silicon macOS release, including the UX improvements and opt-in
native LiteRT upscaling. The signed 0.1.9 → 0.1.10 updater/relaunch path passed;
broader machine, provider, accessibility, and image-quality acceptance remains open.

Start with [Project status and completion plan](docs/PROJECT_STATUS.md) for the
current evidence, sequenced work, owners, dependencies, and acceptance checklists.
[Release operations](docs/RELEASE_OPERATIONS.md) records publication evidence,
build/update testing, and rollback. Do not infer public release failure from absent
local build outputs after cleanup.

## Product capabilities

- Manage X/Twitter, Facebook Page, Instagram, Mastodon, and TikTok accounts from one desktop app.
- Draft, preview, validate, schedule, publish, duplicate, filter, retry, and bulk-delete posts.
- Use account-specific post versions, labels, local media, stock media, and transient Klipy GIF references.
- Review scheduled work in month, day, and week calendar views.
- Import provider audience and post metrics into local dashboards and reports.
- Run durable publishing and import jobs with rate-limit deferral and failure recovery while the app is open.
- Store app state in SQLite, media in the app-data directory, and secrets in the macOS Keychain.
- Back up and restore app data, inspect redacted logs, receive desktop notifications, and install signed updates. The app quietly checks the signed GitHub release feed once when it opens; downloading and installation remain operator approved.
- Use opt-in, on-device media tools for image upscaling, quality preflight, crop suggestions, local media search, and editable alt-text drafts.

See [docs/FEATURES.md](docs/FEATURES.md) for the provider matrix, current limitations, and deferred work. The canonical operator journeys and their regression IDs are in [docs/USER_FLOWS.md](docs/USER_FLOWS.md).

## Development

Prerequisites:

- Apple Silicon macOS for the supported MVP packaging path.
- Node.js and npm.
- Rust and Cargo.
- The platform prerequisites required by Tauri v2.
- PHP and Composer only when working on the retained Mixpost package.

Install dependencies and run the desktop app:

```sh
npm ci
npm run desktop:dev
```

On macOS, `desktop:dev` signs the changing debug executable with the installed Developer ID identity and the stable `com.dustwave.social` identifier before launch. This keeps its Keychain access requirement aligned with the production app. If no Developer ID identity is available, debug builds default to environment-only credentials instead of repeatedly prompting for Keychain access. Do not override `DUSTWAVE_KEYCHAIN_MODE=keychain` for an ad-hoc-signed executable.

Run the release-oriented verification suite:

```sh
npm run desktop:release:check
```

Run the legacy Mixpost asset build or PHP tests only when changing that package:

```sh
npm run build
composer test
```

## Repository map

- `resources/desktop/`: Vue desktop interface and bundled local-AI assets.
- `src-tauri/`: Rust commands, provider adapters, SQLite repositories, migrations, packaging, and permissions.
- `workers/tiktok-broker/`: Cloudflare Worker that isolates TikTok OAuth secrets and imports analytics.
- `workers/media-staging/`: Cloudflare Worker and R2 binding for temporary Instagram media URLs.
- `scripts/`: build, release, signing, notarization, updater, provider setup, and readiness automation.
- `src/`, `resources/js/`, `routes/`, `database/`, and `tests/`: retained Mixpost Lite package.

## Documentation

- [Features and limits](docs/FEATURES.md)
- [User flows and UX regression matrix](docs/USER_FLOWS.md)
- [Project status, remaining work, and completion checklist](docs/PROJECT_STATUS.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Provider integration setup](docs/PROVIDER_SETUP.md)
- [Release operations](docs/RELEASE_OPERATIONS.md)
- [Local AI media](docs/LOCAL_AI.md)
- [Product and ethical safeguards](docs/BEST_PRACTICES.md)
- [Support runbook](docs/SUPPORT_RUNBOOK.md)
- [Security policy](SECURITY.md)
- [Third-party notices](docs/THIRD_PARTY_NOTICES.md)
- [Changelog](docs/CHANGELOG.md)

Keep only the project entry point, license, and security reporting policy at the
repository root. Maintained guides, release history, and dependency notices live
in `docs/`; component READMEs stay alongside their code. Each document has one
purpose—link to its authoritative content instead of copying it.

Validate the documentation structure and relative links after changing docs:

```sh
npm run docs:check
```

For an ephemeral local release-readiness snapshot, run `npm run mvp:release:notes`
and then `npm run mvp:release:notes:check`. The output is ignored under
`artifacts/release-readiness.md`, not committed documentation.

## Project boundaries

The MVP targets Apple Silicon macOS. TikTok direct API publishing, richer Instagram formats, Facebook Groups, Intel/universal builds, cloud sync, and team collaboration are not part of the current release scope.

Mixpost-originated PHP code remains MIT licensed under [LICENSE.md](LICENSE.md). Dust Wave release bundles must also comply with the notices and redistribution requirements in [Third-party notices](docs/THIRD_PARTY_NOTICES.md).
