# Provider Integration Setup

Updated: 2026-08-29

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
- Login model: `Facebook Login for Business · user access token configuration`
- Connection + import permissions: `pages_show_list,pages_read_engagement,read_insights`
- Dust Wave configuration field: **Login configuration ID**
- Default API version: `v25.0`
- Dust Wave credential fields: **App ID** and **App Secret**

In Meta App Dashboard, add the Pages API use case and add `pages_show_list`, `pages_read_engagement`, and `read_insights` under **Manage everything on your Page > Permissions and features**. Under **Facebook Login for Business > Configurations**, create one configuration with a clear client name, choose **General**, choose **User access token**, and select those same three permissions. Copy its non-secret Configuration ID into Provider setup. Dust Wave uses `config_id` rather than sending a second raw OAuth scope list, so the portal configuration remains the permission authority. Add the Instagram Business use case only when connecting a professional Instagram account, and add `pages_manage_posts` only before a deliberate Facebook publishing test.

A personal Facebook profile is the OAuth identity, but Dust Wave Social connects Facebook Pages and Instagram Business or Creator accounts returned through that identity; it does not publish to a personal Facebook timeline. Standard access is enough only for people with an app role. Keep the app unpublished while testing controlled identities, and request Advanced Access before onboarding people without an app role. Recheck the live Meta console before changing review or publication state because provider requirements change.

After saving the Meta credentials, select **Connect Facebook or Instagram**. Dust Wave switches directly to the matching account form. Authorize the controlled identity, choose **current Pages only**, select only the intended Page or professional Instagram account, and run Refresh before testing an insights import or post.

Facebook imports `page_post_engagements` and `page_media_view`. Do not restore `page_posts_impressions`: Meta deprecated it for every Graph API version, and the provider now returns an invalid-metric error when it is requested. See Meta's [deprecated Facebook Page Insights metrics](https://developers.facebook.com/docs/platforminsights/page/deprecated-metrics).

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

- App platforms: `Web and Desktop`
- Official website: `https://dustwave.xyz/social/`
- Public Terms URL: `https://dustwave.xyz/social/terms`
- Public Privacy URL: `https://dustwave.xyz/social/privacy`
- URL verification: `Verify dustwave.xyz once with a DNS TXT record; this covers the website, Terms, and Privacy URLs.`
- Analytics product: `Login Kit; add analytics permissions under Scopes because Display API is not a separate product in the current portal.`
- Login Kit Web redirect URI: `https://dustwave-tiktok-broker.jogo.workers.dev/api/tiktok/oauth/callback`
- MVP analytics scopes: `user.info.basic,user.info.stats,video.list`
- Future publishing scopes: `video.upload,video.publish`
- First review gate: `Configure and save Sandbox first, then use Production > Import > Import from Sandbox; save the Production draft, then add a real target account and demo video before review.`
- Desktop credential: **Client Key** only
- Broker URL: `https://dustwave-tiktok-broker.jogo.workers.dev`
- Publishing mode: **Assisted**
- Client secret storage: `Store TikTok client secret only in the Cloudflare broker, never in this desktop app.`

Use this portal order so the same values are entered only once:

1. In **URL properties**, choose **Domain**, enter `dustwave.xyz`, and add the generated TXT record at the DNS provider. In Cloudflare, use **TXT**, name `@`, and **Auto** TTL. Return to the existing `dustwave.xyz` row under **Unverified properties**, choose **Continue to verify**, and finish that challenge instead of creating a second one. Domain verification covers the website, Terms, and Privacy URLs together.
2. In **Basic information**, upload the 1024px app icon, select **Web** and **Desktop**, and enter the three public URLs above.
3. Add **Login Kit**. The current portal exposes the Display API permissions through **Scopes**, rather than as a second product, so add only `user.info.basic`, `user.info.stats`, and `video.list`.
4. In Login Kit, select the **Web** redirect tab and add the broker callback. Do not put the remote HTTPS broker URL under **Desktop**; that tab accepts loopback callbacks such as `localhost` or `127.0.0.1` instead.
5. Create a clearly named Sandbox such as **Dust Wave Social Test**, configure the same app details, Login Kit callback, and scopes there, then click **Apply changes**. In **Sandbox settings**, choose **Add account** and authenticate a real TikTok consumer account; the developer-portal login is not automatically a TikTok target account.
6. In Production, choose **Import > Import from Sandbox > Dust Wave Social Test** so the verified Sandbox contract is reused instead of re-entered. Add the review explanation and save the Production draft. Record and upload the real end-to-end Sandbox demo only after the controlled target account works. Do not submit the draft for review until every checklist item below is true.

Direct posting remains disabled until TikTok approves the relevant Content Posting API scope and the product deliberately enables that mode.

The **Client Secret** goes only into the Cloudflare broker secret named `TIKTOK_CLIENT_SECRET`. The same **Client Key** is saved as broker secret `TIKTOK_CLIENT_KEY` and in Dust Wave Social. Never store the Client Secret in the desktop app.

The Social-specific policy pages are public and intentionally use stable extensionless URLs. They do not need permanent links in the global Dust Wave footer; keep the provider-facing URLs synchronized with the deployed pages instead of substituting temporary documents or repository links.

Before clicking **Submit for review**, confirm all of the following:

- The app name, icon, category, description, Web and Desktop platforms, public website, Terms URL, and Privacy URL are saved.
- `dustwave.xyz` is verified once at the domain level, covering all three public URLs.
- Login Kit and every requested scope appear in the portal.
- The broker callback matches exactly under Login Kit's Web redirect tab.
- A controlled TikTok consumer account appears under Sandbox target users.
- A Sandbox demo video shows the real Dust Wave Social authorization and analytics flow for the controlled account.
- The review explanation covers every selected product and scope and does not claim direct publishing when only Assisted mode is implemented.

For Sandbox acceptance, deploy the broker with the Sandbox client key and secret, save only that same non-secret Client Key in Dust Wave Social, and open the broker authorization URL from Provider setup. Authorize a Sandbox target account, then copy the returned TikTok user ID, display name, optional username, granted scopes, and opaque connection credential into **Add account > TikTok**. Run **Import** immediately. A newly created account may legitimately return `0 videos · 0 metric days`; a completed import without a broker or authorization error still proves the controlled Sandbox path.

Record that real flow for the review demo. After Production approval, replace the broker's Sandbox client credentials and the desktop Client Key with the Production values before authorizing accounts outside the Sandbox target-user list.

## Unsplash

Open <https://unsplash.com/oauth/applications>, sign in with an account you control, create a clearly named application, and accept the current API guidelines.

On **New API Application**, use the product name and describe the actual stock-search flow. After creation, keep the app in Demo mode for controlled testing and copy the **Access Key** from the Keys section. Do not copy the **Secret key**. Dust Wave Social does not need a redirect URI for its public stock-search flow.

Exact setup value:

- Required access: `Public demo or production access key`
- Dust Wave credential field: **API Key**

Save the public access key, activate Unsplash, open Media > Stock, run one controlled search, and confirm attribution plus any download-trigger requirements before wider use. Do not copy a secret key into the desktop field if the provider presents both public and secret values.

## Klipy

Open <https://partner.klipy.com/> and create a Partner Panel app under an account you control.

Use **Add Platform**, name the platform for your product, provide its public product or policy URL, and accept the current API terms yourself. On **Create API Key**, use a key name that identifies the client, such as `Product name macOS`. Leave Ads API off unless the product deliberately implements KLIPY ads and their additional data flow.

Exact setup values:

- Access path: `Create a Partner Panel app, test with 100 calls/hour, then request production access.`
- Attribution: `Dust Wave Social supplies “Search KLIPY” and “Powered by KLIPY” with the official logo; preserve this treatment and the provided content attribution.`
- Dust Wave credential field: **API Key**

Save the test key and activate Klipy. Dust Wave Social supplies the required search placeholder and bundled attribution treatment; do not remove or restyle them into invisibility. Open Media > GIFs and run one controlled search. Klipy media stays a provider reference and may be materialized only as a temporary publish-time file; it must not be added to the reusable local media library. Review [GIF_PROVIDER_DECISION.md](GIF_PROVIDER_DECISION.md) before production use.

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
