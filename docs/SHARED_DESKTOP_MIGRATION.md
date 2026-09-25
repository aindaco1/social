# Shared desktop services migration

Source migration only; release and deployment acceptance remain separate.

- Consumer baseline: `3b3ed40754fb7cadb8b79105b8de371b35877464`.
- Previous Platform pin: `none (new submodule)`.
- New immutable commit and exact versions: [platform-desktop.json](../platform-desktop.json).
- Shared surface: Tauri progress accounting and release-manifest helpers.

Keep Vue status text and Rust check/install/restart commands, version binding, signature verification and consent. Local health export and social metrics remain product-owned.

## Validation

Before and after: three fixtures exercise the actual updater callbacks for silent checks, progress/version binding and failed-install retry. After: desktop Vite build, 163 UI tests, and all three release-packaging checks passed.

All consumer gitlinks, exact package versions and retained Sparkle lockfile
revisions pass:

```sh
node shared/dust-wave-platform/scripts/check-desktop-consumer.mjs
```

Platform passes its JavaScript suite and clean-checkout recipe tests. Its seven
desktop Swift tests pass independently with Sparkle 2.9.5, 2.9.6 and 2.10.0.
App manifests retain their exact existing Sparkle revisions. Advancing the
full gitlink also carries existing Platform patches; product-owned tests
cover those dependencies.

## Independent rollback

Revert this repository's migration commit, then run
`git submodule update --init --recursive`. This restores the prior adapters,
dependency declaration, gitlink and build/CI configuration together. For a
newly added Platform submodule, Git may leave an untracked checkout directory;
it is no longer a build input after the revert.

No user data or relay storage migration is required. Other applications may
stay on their chosen Platform revisions. A reverse-patch check of the complete
migration records whether the source rollback applies cleanly.

Local source/build evidence does not establish notarization, a signed updater
replacement, physical hardware behavior or deployed GitHub delivery. Use the
existing release runbook before shipping.
