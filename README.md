# Dust Wave Social

Dust Wave Social is a local-first social publishing and reporting app for Apple Silicon macOS. It is built with Tauri, Rust, Vue, and SQLite, and keeps provider credentials in the macOS Keychain instead of the local database.

The repository also retains the original Mixpost Lite Laravel package while the desktop replacement completes live-provider and release acceptance. The desktop app does not require the Laravel server for normal operation.

## Current status

Version 0.1.3 is published for Apple Silicon macOS as a signed, notarized, and stapled DMG. It fixes the Vue/Tauri integration bug that prevents versions 0.1.0 through 0.1.2 from completing an in-app install. Those affected versions require one manual 0.1.3 DMG installation without uninstalling the existing app first.

The protected release workflow installed the public signed 0.1.3 update into a staged 0.1.2 app and verified the resulting bundle version. Version 0.1.3 also adds UI-level coverage for the Vue/Tauri resource boundary; hands-on installation of the hotfix and the next in-app UI update hop remain operator acceptance items. Production provider credentials, real-account publishing/import acceptance, packaged local-AI review, and an independent clean-Mac install also still require operator evidence.

Run the current readiness audit instead of copying status counts into another document:

```sh
npm run mvp:launch:readiness
```

The launch source of truth is [docs/MVP_LAUNCH_PLAN.md](docs/MVP_LAUNCH_PLAN.md). It records the current artifact state, manual acceptance gates, release commands, and rollback procedure.

## Product capabilities

- Manage X/Twitter, Facebook Page, Instagram, Mastodon, and TikTok accounts from one desktop app.
- Draft, preview, validate, schedule, publish, duplicate, filter, retry, and bulk-delete posts.
- Use account-specific post versions, labels, local media, stock media, and transient Klipy GIF references.
- Review scheduled work in month, day, and week calendar views.
- Import provider audience and post metrics into local dashboards and reports.
- Run durable publishing and import jobs with rate-limit deferral and failure recovery while the app is open.
- Store app state in SQLite, media in the app-data directory, and secrets in the macOS Keychain.
- Back up and restore app data, inspect redacted logs, receive desktop notifications, and install signed updates.
- Use opt-in, on-device media tools for image upscaling, quality preflight, crop suggestions, local media search, and editable alt-text drafts.

See [docs/FEATURES.md](docs/FEATURES.md) for the provider matrix, current limitations, and deferred work.

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
- [Architecture](docs/ARCHITECTURE.md)
- [MVP launch and release operations](docs/MVP_LAUNCH_PLAN.md)
- [Local AI media](docs/LOCAL_AI.md)
- [GIF provider decision](docs/GIF_PROVIDER_DECISION.md)
- [Product and ethical safeguards](docs/BEST_PRACTICES.md)
- [Support runbook](docs/SUPPORT_RUNBOOK.md)
- [Security policy](SECURITY.md)
- [Third-party notices](THIRD_PARTY_NOTICES.md)
- [Changelog](CHANGELOG.md)

Validate the documentation structure and relative links after changing docs:

```sh
npm run docs:check
npm run mvp:release:notes:check
```

## Project boundaries

The MVP targets Apple Silicon macOS. TikTok direct API publishing, richer Instagram formats, Facebook Groups, Intel/universal builds, cloud sync, and team collaboration are not part of the current release scope.

Mixpost-originated PHP code remains MIT licensed under [LICENSE.md](LICENSE.md). Dust Wave release bundles must also comply with the notices and redistribution requirements in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
