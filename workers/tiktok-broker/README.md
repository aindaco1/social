# Dust Wave TikTok Broker

This Worker keeps TikTok OAuth client secrets, refresh tokens, and access tokens out of the desktop app. Dust Wave Social stores only an opaque broker connection credential in the local keychain.

## Current Dust Wave Scaffold

- Worker URL: `https://dustwave-tiktok-broker.jogo.workers.dev`
- Health check: `https://dustwave-tiktok-broker.jogo.workers.dev/api/health`
- D1 database: created.
- Initial migration: applied.
- Generated secrets already stored in local `Apple Auth`, GitHub secrets, and Cloudflare Worker secrets: `TOKEN_ENCRYPTION_KEY`, `BROKER_ADMIN_TOKEN`.
- Still required before live TikTok acceptance: `TIKTOK_CLIENT_KEY`, `TIKTOK_CLIENT_SECRET`, TikTok OAuth callback registration, and a D1-capable `CLOUDFLARE_API_TOKEN` for GitHub Actions deploys.

## Endpoints

- `GET /api/health`
- `GET /api/tiktok/oauth/start` redirects to TikTok Login Kit.
- `GET /api/tiktok/oauth/callback` exchanges the authorization code, stores encrypted tokens in D1, and shows the account values to paste into Dust Wave Social.
- `GET /api/tiktok/accounts/:open_id/analytics` returns the user stats and video list used by the desktop analytics importer. The request must include `Authorization: Bearer <broker connection credential>`.
- `GET /api/tiktok/accounts/:open_id/status` returns non-secret account and broker-connection status. The request must include `Authorization: Bearer <broker admin token>`.
- `POST /api/tiktok/accounts/:open_id/revoke-connections` revokes active desktop broker credentials for the account. The request must include `Authorization: Bearer <broker admin token>`.

## Configure

1. Create the D1 database:
   ```sh
   npx wrangler d1 create dustwave-tiktok-broker
   ```

2. Export the returned `database_id`. This value is not a secret, but keep it out of screenshots and public setup packets:
   ```sh
   export TIKTOK_BROKER_D1_DATABASE_ID="<database-id-from-wrangler>"
   ```

3. Generate a local Wrangler config from environment values:
   ```sh
   npm run tiktok:broker:config:check
   ```

   The generated config is written to `workers/tiktok-broker/wrangler.generated.jsonc` and is intentionally ignored by git. The checked-in `wrangler.toml` remains a readable template.

4. Generate a 32-byte encryption key:
   ```sh
   openssl rand -base64 32
   ```

5. Set Worker secrets:
   ```sh
   npx wrangler secret put TIKTOK_CLIENT_KEY --config workers/tiktok-broker/wrangler.generated.jsonc
   npx wrangler secret put TIKTOK_CLIENT_SECRET --config workers/tiktok-broker/wrangler.generated.jsonc
   npx wrangler secret put TOKEN_ENCRYPTION_KEY --config workers/tiktok-broker/wrangler.generated.jsonc
   npx wrangler secret put BROKER_ADMIN_TOKEN --config workers/tiktok-broker/wrangler.generated.jsonc
   ```

6. Apply migrations:
   ```sh
   npm run tiktok:broker:migrate
   ```

7. Deploy:
   ```sh
   npm run tiktok:broker:deploy
   ```

8. In TikTok Developer Portal, register the broker callback URL:
   ```text
   https://your-worker-or-custom-domain/api/tiktok/oauth/callback
   ```

9. In Dust Wave Social, set Services -> TikTok -> Broker URL to the Worker origin, for example:
   ```text
   https://dustwave-tiktok-broker.your-subdomain.workers.dev
   ```

## GitHub Actions Deployment

The workflow in `.github/workflows/tiktok-broker.yml` runs broker tests on pull requests and pushes. It can also deploy the Worker manually from GitHub Actions after these repository settings exist:

- Secrets: `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`, `TIKTOK_BROKER_D1_DATABASE_ID`, `TIKTOK_CLIENT_KEY`, `TIKTOK_CLIENT_SECRET`, `TOKEN_ENCRYPTION_KEY`, `BROKER_ADMIN_TOKEN`
- Optional variables: `TIKTOK_BROKER_WORKER_NAME`, `TIKTOK_BROKER_D1_DATABASE_NAME`, `PUBLIC_BROKER_BASE_URL`, `TIKTOK_SCOPES`

Plan the GitHub settings without printing secret values:

```sh
npm run tiktok:broker:github:secrets:plan -- --repo aindaco1/social
```

Apply settings from the current shell environment:

```sh
npm run tiktok:broker:github:secrets:plan -- --repo aindaco1/social --apply
```

You can also source values from a local env file:

```sh
npm run tiktok:broker:github:secrets:plan -- --repo aindaco1/social --env-file workers/tiktok-broker/.dev.vars --apply
```

Then run the `TikTok Broker` workflow with `deploy_broker=true`. Use `apply_migrations=true` for the first production deploy or after adding a migration.

## Local Development

Copy `.dev.vars.example` to `.dev.vars`, fill in local values, and run:

```sh
npm run tiktok:broker:migrate:local
npm run tiktok:broker:dev
```

Run the broker boundary tests without live TikTok or Cloudflare credentials:

```sh
npm run tiktok:broker:test
```

The MVP scopes are `user.info.basic,user.info.stats,video.list`. Direct publishing still requires TikTok approval for `video.upload` or `video.publish`.

## Revocation

Broker connection credentials are stored only as SHA-256 hashes. To revoke every desktop credential for a TikTok account:

```sh
curl -X POST \
  -H "Authorization: Bearer $BROKER_ADMIN_TOKEN" \
  "https://your-worker-or-custom-domain/api/tiktok/accounts/<tiktok-open-id>/revoke-connections"
```

To inspect non-secret broker status for a TikTok account:

```sh
curl \
  -H "Authorization: Bearer $BROKER_ADMIN_TOKEN" \
  "https://your-worker-or-custom-domain/api/tiktok/accounts/<tiktok-open-id>/status"
```
