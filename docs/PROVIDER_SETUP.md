# Provider Integration Setup

Updated: 2026-08-28

Audience: people configuring Dust Wave Social for accounts they control.

This is the user-facing setup guide for Connections > Provider setup and Connections > Connected accounts. The provider catalog in [`resources/desktop/src/providerSetup.js`](../resources/desktop/src/providerSetup.js) is the application source of truth for portal URLs, callback URLs, scopes, credential fields, and defaults. The documentation check fails when the exact setup values below drift from that catalog.

## The five-step path

For each provider:

1. Open Connections > Provider setup and select the provider.
2. Open the developer portal and create or select a clearly named test app under an account you control.
3. Use **Copy Exact Setup** to transfer callback URLs, scopes, and policy notes. Copied packets never contain credential values.
4. Enter the returned key or ID and secret in Dust Wave Social, select **Active**, and use the single **Save Settings** action. Secrets are stored in macOS Keychain.
5. Use **Connect account** from the same setup card, authorize a controlled test account, refresh it, and run one provider-appropriate import or media search before adding organization accounts.

Keep provider setup, account authorization, and live publishing as separate checks. A saved key does not prove that OAuth, posting, imports, app review, or the provider tier works.

## Test-account policy

- Start with a personal or dedicated test account that you control. Name developer apps with `Test` or `Sandbox` when the provider permits it.
- Do not authorize an organization account until the controlled account can connect, refresh, import, and recover from an expected error.
- Use provider Development or Sandbox mode while it covers the test account. Move to Live or Production review only when outside users require it and every public policy, review, and demo requirement is ready.
- Use a small, unmistakable post for publishing acceptance and remove it manually after recording the provider result. Dust Wave Social never treats a local queue result as proof that a provider published.
- Never paste a provider access token, refresh token, client secret, one-time code, or broker credential into documentation, an issue, a screenshot, or a copied setup packet.

## Facebook

Create or select the Meta app at <https://developers.facebook.com/apps>. Dust Wave Social uses one Meta app for Facebook Pages and Instagram professional accounts.

Exact desktop values:

- OAuth callback URL: `http://localhost/callback`
- Default API version: `v25.0`
- Default OAuth scopes: `business_management,pages_show_list,read_insights,pages_manage_posts,pages_read_engagement,pages_manage_engagement,instagram_basic,instagram_content_publish,instagram_manage_insights,instagram_manage_comments`
- Dust Wave credential fields: **App ID** and **App Secret**

Use the minimum provider use cases and permissions that match the flow being tested. A personal Facebook profile is the OAuth identity, but Dust Wave Social connects Facebook Pages and Instagram Business or Creator accounts returned through that identity; it does not publish to a personal Facebook timeline. Keep the app in the provider's limited test/development state when the test identity and assets are eligible. Recheck the live Meta console before changing review or publication state because provider requirements change.

After saving the Meta credentials, select **Connect Facebook or Instagram**, authorize the controlled identity, choose only the intended Page or professional Instagram account, and run Refresh before testing a post or insights import.

## Instagram Local Media

Exact managed-service values:

- Worker: `dustwave-media-staging`
- R2 bucket: `dustwave-media-staging`
- Retention: `Temporary high-entropy URLs with 24 hour default TTL`

Ordinary users do not need Cloudflare, Wrangler, an R2 bucket name, or a reusable operator token.

1. A provisioned operator runs `npm run media:staging:enrollment -- --label "Name of Mac"`.
2. The 15-minute, one-use code is transferred privately.
3. The user opens Connections > Provider setup > Instagram Local Media, pastes it under **Pair this Mac**, and selects **Pair This Mac**.
4. The user connects an Instagram professional account and publishes one controlled static JPEG or PNG.
5. The operator confirms that the temporary object is deleted after the attempt or by scheduled cleanup.

Advanced recovery keeps the service URL at `https://dustwave-media-staging.jogo.workers.dev`. Do not make the R2 bucket public.

## X

Open <https://developer.twitter.com/en/portal/projects-and-apps> and select the app used for the controlled account.

Exact values:

- OAuth callback URL: `http://localhost/callback`
- OAuth scopes: `tweet.read tweet.write users.read offline.access`
- Dust Wave credential fields: **API Key** and **API Secret**
- Default tier setting: **Pay as you go**

Paste-ready use-case description:

> I use Dust Wave Social, a personal desktop application, only to authenticate accounts I control and publish posts that I explicitly write, review, and submit. I do not resell provider data, scrape users, automate spam, or use provider data for surveillance or profiling.

After saving the credentials, connect the personal test account, complete PKCE authorization, refresh the account, and verify the configured tier supports the intended post, media, read/import, and rate-limit behavior.

## TikTok

Open <https://developers.tiktok.com/>. Use Sandbox for the first end-to-end demonstration when the portal requires it; do not submit a Production review until public legal URLs and the required demo video are ready.

Exact values:

- Official website: `https://dustwave.xyz/social/`
- Public Terms URL: `https://dustwave.xyz/social/terms`
- Public Privacy URL: `https://dustwave.xyz/social/privacy`
- Broker callback URL: `https://dustwave-tiktok-broker.jogo.workers.dev/api/tiktok/oauth/callback`
- MVP analytics scopes: `user.info.basic,user.info.stats,video.list`
- Future publishing scopes: `video.upload,video.publish`
- Desktop credential: **Client Key** only
- Broker URL: `https://dustwave-tiktok-broker.jogo.workers.dev`
- Publishing mode: **Assisted**
- Client secret storage: `Store TikTok client secret only in the Cloudflare broker, never in this desktop app.`

Add **Login Kit**, then request only the MVP analytics scopes used by the current broker flow. Direct posting remains disabled until TikTok approves the relevant Content Posting API scope and the product deliberately enables that mode.

The **Client Secret** goes only into the Cloudflare broker secret named `TIKTOK_CLIENT_SECRET`. The same **Client Key** is saved as broker secret `TIKTOK_CLIENT_KEY` and in Dust Wave Social. Never store the Client Secret in the desktop app.

The Social-specific policy pages are public, linked from the Dust Wave site footer, and intentionally use stable extensionless URLs. Keep those provider-facing URLs synchronized with the deployed pages instead of substituting temporary documents or repository links.

Before clicking **Submit for review**, confirm all of the following:

- The app name, icon, category, description, Desktop platform, public Terms URL, and public Privacy URL are saved.
- Login Kit and every requested scope appear in the portal.
- The broker callback matches exactly.
- A Sandbox demo video shows the real Dust Wave Social authorization and analytics flow for the controlled account.
- The review explanation covers every selected product and scope and does not claim direct publishing when only Assisted mode is implemented.

After approval and broker deployment, open the broker authorization URL, authorize the controlled TikTok account, and paste only the broker-issued opaque connection credential into Dust Wave Social.

## Unsplash

Open <https://unsplash.com/oauth/applications>, sign in with an account you control, create a clearly named application, and accept the current API guidelines.

Exact setup value:

- Required access: `Public demo or production access key`
- Dust Wave credential field: **API Key**

Save the public access key, activate Unsplash, open Media > Stock, run one controlled search, and confirm attribution plus any download-trigger requirements before wider use. Do not copy a secret key into the desktop field if the provider presents both public and secret values.

## Klipy

Open <https://partner.klipy.com/> and create a Partner Panel app under an account you control.

Exact setup values:

- Access path: `Create a Partner Panel app, test with 100 calls/hour, then request production access.`
- Attribution: `Keep “Search KLIPY”; display “Powered by KLIPY” with the official logo and provided content attribution before API use.`
- Dust Wave credential field: **API Key**

Save the test key, add the required attribution treatment, activate Klipy, open Media > GIFs, and run one controlled search. Klipy media stays a provider reference and may be materialized only as a temporary publish-time file; it must not be added to the reusable local media library. Review [GIF_PROVIDER_DECISION.md](GIF_PROVIDER_DECISION.md) before production use.

## Mastodon

Mastodon has no shared Provider setup card. Open Connections > Connected accounts > Add account > Mastodon and enter the server origin, such as `https://mastodon.social`, not a profile URL.

Dust Wave Social dynamically registers a server-specific app, stores its client secret in macOS Keychain, opens that server's OAuth page, and connects the returned code. Repeat registration for each distinct server. Confirm that the server permits dynamic app registration and review its media, character, rate-limit, and moderation rules before publishing.

## Acceptance record

For every integration, record evidence without secrets:

| Checkpoint | Record |
| --- | --- |
| Provider setup | Provider, developer-app name, account owner, test/production mode, callback, requested scopes, and date checked |
| Desktop setup | Dust Wave Social version, service Active state, and credential fields reported available |
| OAuth | Controlled account handle, authorization result, returned asset selection, and visible error if blocked |
| Import/search | Action, provider response, item or metric count, and visible attribution/policy result |
| Publishing | Test format, provider-returned ID or URL, visible public result, cleanup result, and timestamp |
| Recovery | Expected failure introduced, operator-facing message, retry/reconnect result, and whether duplicate publishing occurred |

Never record passwords, API secrets, access/refresh tokens, one-time codes, broker credentials, or Keychain values.

## Adding another provider

Keep new integrations data-driven:

1. Add the portal URL, callbacks, scopes, credential fields, defaults, and account mapping once in `resources/desktop/src/providerSetup.js`.
2. Keep provider secrets in Keychain or a server-side broker; persist only non-secret configuration and secret references in SQLite.
3. Reuse the guided setup card, copied secret-free packet, one Save Settings action, and Add account flow.
4. Add provider-specific code only for OAuth, capability rules, publishing/import behavior, and real provider errors.
5. Add a user-flow contract and update this guide. Run `npm run docs:check`, `npm run desktop:ui:test`, and the applicable Rust or Worker tests.
6. Keep local tests, provider acceptance, app review, deployment, and real public results as separate status claims.
