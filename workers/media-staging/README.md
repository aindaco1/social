# Dust Wave Media Staging Worker

This Cloudflare Worker gives providers such as Instagram a short-lived public HTTPS URL for media that originates in the local desktop app. Objects remain private in R2 and are served only through the Worker.

## Endpoints

- `GET /api/health`: non-secret health response.
- `POST /api/media/stage`: authenticated media upload to R2.
- `GET /media/:key`: public read of an unexpired staged object.
- `POST /api/media/cleanup`: authenticated deletion of expired objects.
- Scheduled handler: hourly cleanup using the configured Cron Trigger.

Stage and cleanup requests use `MEDIA_STAGING_TOKEN`. Public object keys are high-entropy and temporary. The default maximum object size is 25 MB, the default lifetime is 24 hours, and requested lifetimes are capped at seven days.

## Configuration

Required deployment inputs:

- `MEDIA_STAGING_TOKEN`: Worker secret and matching desktop Keychain value.
- `MEDIA_STAGING_BUCKET_NAME`: R2 bucket name.
- `PUBLIC_MEDIA_BASE_URL`: HTTPS Worker origin.
- `CLOUDFLARE_ACCOUNT_ID` and a deployment token for Wrangler or GitHub Actions.

Optional inputs:

- `MEDIA_STAGING_WORKER_NAME`.
- `MEDIA_STAGING_MAX_OBJECT_BYTES`.
- `MEDIA_STAGING_DEFAULT_TTL_SECONDS`.
- `MEDIA_STAGING_CLEANUP_CRON`.

Generate and validate the ignored Wrangler configuration:

```sh
npm run media:staging:config
npm run media:staging:wrangler:check
```

For local development, copy `.dev.vars.example` to `.dev.vars`, provide local values, and run:

```sh
npm run media:staging:dev
```

## Test and deploy

```sh
npm run media:staging:test
npm run media:staging:deploy
```

The GitHub Actions workflow in `.github/workflows/media-staging.yml` runs boundary tests, performs a Wrangler dry run when configuration is available, and supports manual deployment.

## Operational rules

- Do not make the R2 bucket public.
- Keep the staging token out of the repository, logs, screenshots, support exports, and setup packets.
- Delete staged media after the provider publish attempt when possible; rely on scheduled cleanup as the recovery path.
- Verify MIME signatures and size limits before accepting an upload.
- Treat a leaked staging token as an incident: rotate the Worker/GitHub value and update the desktop Keychain value before resuming Instagram publishing.

Operator troubleshooting is in [../../docs/SUPPORT_RUNBOOK.md](../../docs/SUPPORT_RUNBOOK.md).
