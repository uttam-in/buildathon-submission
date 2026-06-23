# Deploying to Google Cloud (project `siemap-500222`)

The Digital Menu Board runs as a single stateless container (the Axum server)
on **Cloud Run**. It serves the public board renderer, the DB-backed store
walls, and the admin console. State lives in managed Postgres (Railway); the
static board assets (`Resources/`) are baked into the image.

## Files

| File | Purpose |
|------|---------|
| `Dockerfile` | Multi-stage build: compiles the Rust workspace, bakes `Resources/`, ships `dmbr-server-axum` + `dmbr-migrate` on a slim Debian runtime. |
| `cloudbuild.yaml` | Cloud Build pipeline: build → push to Artifact Registry → run migrations as a Cloud Run job → deploy the service. |
| `deploy.sh` | Idempotent one-shot bootstrap: enables APIs, creates the AR repo + secrets, grants IAM, submits the build. |
| `../.dockerignore`, `../.gcloudignore` | Trim the build/upload context (exclude `target/`, `node_modules/`, `.env`). |

> **Build context is the submission root** (`buildathon-submission/`). The
> static board assets live in the vendored `Resources/` directory there, so
> the build is fully self-contained — no sibling directories required.

## Runtime configuration

| Variable | Source | Notes |
|----------|--------|-------|
| `PORT` | Cloud Run (injected) | Server binds `0.0.0.0:$PORT`. |
| `RESOURCES_DIR` | env var → `/app/Resources` | Baked into the image. |
| `DATABASE_URL` | Secret `dmbr-database-url` | Postgres DSN. |
| `SESSION_SECRET` | Secret `dmbr-session-secret` | **≥ 32 chars** or the server refuses to start. |
| `ADMIN_USER` / `ADMIN_PASSWORD` | Secrets `dmbr-admin-user` / `dmbr-admin-password` | Used **only** by the migrate job to seed the admin login. |

## First-time deploy

```bash
# From the submission root (buildathon-submission/). Provide secrets via env —
# they are written to Secret Manager, never committed.
DATABASE_URL='postgres://USER:PASS@HOST:PORT/DB' \
SESSION_SECRET="$(openssl rand -hex 24)"          \
ADMIN_USER='admin'                                \
ADMIN_PASSWORD='<strong password>'                \
./deploy/deploy.sh
```

`deploy.sh` is idempotent — re-run it any time. Omit the secret env vars on
later runs to keep the existing values; pass one to rotate it.

## Subsequent deploys (code change only)

```bash
# From the submission root (buildathon-submission/):
gcloud builds submit . \
  --config deploy/cloudbuild.yaml \
  --project siemap-500222
```

This rebuilds, re-runs migrations (idempotent — `IF NOT EXISTS` + admin
upsert + seed-only-if-empty), and rolls out a new Cloud Run revision.

## What the migrate step does

The pipeline runs the `dmbr-migrate` binary as a Cloud Run **job** before the
service deploy. It applies `migrations/0001..0003` and seeds the menu from the
baked `Resources/menu.json` (only if the menu table is empty) and the admin
user. Re-running is safe.

## After deploy

```bash
# Service URL
gcloud run services describe dmbr --region us-central1 \
  --project siemap-500222 --format 'value(status.url)'
```

- Public boards:  `<URL>/`  and  `<URL>/store/<slug>`
- Admin console:  `<URL>/admin`  (log in with `ADMIN_USER` / `ADMIN_PASSWORD`)

## Rollback

```bash
gcloud run revisions list --service dmbr --region us-central1 --project siemap-500222
gcloud run services update-traffic dmbr --region us-central1 \
  --to-revisions <PREVIOUS_REVISION>=100 --project siemap-500222
```

## Cost shape

- Cloud Run scales to **zero** (`--min-instances=0`); you pay per request.
- One small image in Artifact Registry.
- Postgres is external (Railway), not billed by GCP.
