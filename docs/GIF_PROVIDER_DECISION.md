# GIF Provider Decision

Reviewed: 2026-07-12
Implementation updated: 2026-07-13

## Decision

Dust Wave Social should ship MVP GIF search with Klipy as the active provider, while keeping manual/local GIF import as a separate first-class workflow.

Tenor is retired Mixpost context only. Do not depend on Tenor for MVP acceptance because Google stopped accepting new Tenor API clients in January 2026 and the third-party API shutdown window has passed.

## Why Klipy

- Klipy provides GIF, sticker, meme, clip, emoji, and optional ads APIs from the same provider surface: https://klipy.com/developers
- Klipy explicitly documents a Tenor migration path, including swapping the base URL to `api.klipy.com`, adding a Klipy API key, updating attribution, and requesting production access.
- Klipy's Partner Panel supports a test key at 100 calls/hour, production access with unlimited calls, content filters, and blocklisted keywords.
- Klipy's public iOS demo app confirms a modern service/repository API shape and a GIF search endpoint under `/api/v1/{api_key}/gifs/search`: https://github.com/KLIPY-com/klipy-ios-demo-app

## Terms Review

The public Klipy Terms of Service were last updated on 2026-06-23 and incorporate separate "KLIPY.COM API Terms (if applicable)": https://klipy.com/support/terms-services

Klipy's public API Terms of Service are now available at https://klipy.com/support/api-terms. Those API Terms allow applications to integrate with Klipy and allow application users to access, select, and post GIFs, clips, stickers, memes, and other Klipy content.

This is enough to support Dust Wave's planned GIF search and posting workflow, but not the current "download selected GIF into the reusable local media library" workflow. The API Terms prohibit caching or storing Klipy Content except search-result thumbnails stored inside the user's app copy. They also prohibit using Klipy Content to compile, build, or expand a database, directory, collection, or third-party collection.

Conclusion:

- Allowed: search, preview, select, and post Klipy GIFs when using the API within Klipy's technical/policy limits.
- Allowed: local search-result thumbnails in the user's app copy.
- Not allowed without written Klipy permission: permanent download/import of Klipy GIF files into Dust Wave's reusable local media library.
- Not allowed without written Klipy permission: building any internal GIF archive, reusable collection, bulk cache, provider-mixed GIF index, or media database from Klipy results.
- Requires implementation care: transient fetch/upload during publishing. The terms permit posting but prohibit storage, so any file materialization for X/Twitter, Facebook, Mastodon, or another provider should be temporary, deleted immediately after the publish attempt, and excluded from backups, media library, support exports, and user-facing reusable media.

## Implementation Rules

- Keep the product source as `gifs`; choose Klipy in the backend provider adapter so the UI and Mixpost parity surface do not depend on provider-specific naming.
- Store the Klipy API key in the OS keychain through the Services screen or via `DUSTWAVE_KLIPY_CLIENT_ID`, `DUSTWAVE_KLIPY_API_KEY`, `KLIPY_CLIENT_ID`, or `KLIPY_API_KEY`.
- Do not pass ad parameters, advertising IDs, device fingerprints, or tracking identifiers for MVP.
- Store only provider metadata needed to re-fetch or publish selected Klipy content, such as provider, id, slug, source URL, media format, dimensions, attribution fields, and selected rendition URL. Do not store the binary Klipy GIF as app media.
- Do not allow the Media Library `Download` action to persist Klipy results. Klipy-selected media should attach to posts as external provider references, not reusable uploaded media records.
- At publish time, fetch Klipy media only into a temporary file or stream needed for the provider upload, then delete it immediately after success/failure. Do not include temporary Klipy files in backups, restore manifests, media cleanup inventories, or support exports.
- Do not mix Klipy search results with other GIF-provider result sets unless Klipy gives explicit written consent.
- Display Klipy attribution/branding in the search UI. Current public guidance says to use `Search KLIPY` as the search field placeholder and recommends Powered by KLIPY logo/watermark use.
- Keep manual/local GIF import independent of Klipy. Operators must be able to import local `.gif` files even if Klipy credentials are absent or Klipy is unavailable.
- Treat Klipy content filters/blocklists as a release setup task, not a hidden app default.

## Similar Public Integrations

- Discourse proxies Klipy search/categories through its server, requires a configured API key, rate-limits user searches, and redacts the API key before returning Klipy JSON: https://github.com/discourse/discourse/blob/2e8b5f94a91c8e80d707145d195a8e263e23d880/app/controllers/gifs_controller.rb
- Odoo Discuss proxies Klipy search/categories and stores favorite Klipy IDs, then re-fetches posts by ID instead of storing GIF binaries: https://github.com/odoo/odoo/blob/c5f1a963e6c65cf67b56b7ca2d4b77de66140e78/addons/mail/controllers/discuss/gif.py
- Zulip maps Klipy search results into preview and insert URLs, using a small preview format for UI and a larger format for insertion/upload: https://github.com/zulip/zulip/blob/5784ebe8ca8aeab0358831295144ed94b1e1481a/web/src/klipy_network.ts
- Ghost's Koenig editor uses Klipy as an editor GIF provider with provider config, API-key handling, content filter, and result objects/URLs: https://github.com/TryGhost/Ghost/blob/db2b1e7173137c7f9763acbb0359118b5417a092/koenig/koenig-lexical/src/utils/services/gif.ts
- Status stores recent/favorite GIF metadata locally, but the sampled implementation stores IDs/URLs/metadata rather than imported binary media: https://github.com/status-im/status-go/blob/559ac37191d98ea8c2b09e8f307b70da5f2c89cc/services/gif/gif.go

The common pattern is search/proxy/select/post by URL or ID, with optional local metadata for favorites/recent items. I did not find a mature comparable project that imports Klipy binaries into a permanent reusable local media library.

## Current Status

- Implemented: Klipy credential definition, Services UI entry, GIF search adapter, Klipy response mapper, tests for Klipy result mapping, and a guard that blocks permanent Klipy GIF downloads into the reusable media library.
- Implemented: selected Klipy results attach to posts as external provider references, appear in composer/post previews, validate through the post content schema, and are fetched only as temporary publish-time upload assets that are deleted after the upload attempt.
- Implemented: local/manual GIF import through the existing app media library, file picker/drop target, MIME detection, local storage, and media filtering.
- Pending: production Klipy account, attribution assets/guidelines, content-filter settings, live API acceptance, and written approval only if Dust Wave wants permanent Klipy media-library imports.
