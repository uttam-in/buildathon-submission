#!/usr/bin/env bash
#
# One-shot GCP bootstrap + deploy for the Digital Menu Board.
#
# Idempotent: safe to re-run. It enables the required APIs, creates the
# Artifact Registry repo and Secret Manager secrets, grants the Cloud Build
# service account the roles it needs, then triggers the Cloud Build pipeline
# (build -> push -> migrate -> deploy to Cloud Run).
#
# Usage (from anywhere; the script cd's to the repo root itself):
#
#   DATABASE_URL='postgres://...'                 \
#   SESSION_SECRET='<at least 32 chars>'          \
#   ADMIN_USER='admin'                            \
#   ADMIN_PASSWORD='<strong password>'            \
#   ./buildathon-submission/deploy/deploy.sh
#
# Env overrides: PROJECT_ID (default siemap-500222), REGION (us-central1),
# REPO (containers), SERVICE (dmbr).
#
# Secrets are only (re)written when the matching env var is provided. On a
# re-run with the secrets already in place you can omit them entirely.

set -euo pipefail

PROJECT_ID="${PROJECT_ID:-siemap-500222}"
REGION="${REGION:-us-central1}"
REPO="${REPO:-containers}"
SERVICE="${SERVICE:-dmbr}"

# Resolve the submission root (the build context: Cargo.toml + Resources/).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "==> Project: ${PROJECT_ID}  Region: ${REGION}  Service: ${SERVICE}"
echo "==> Build context: ${REPO_ROOT}"
[ -d "${REPO_ROOT}/Resources" ] || { echo "ERROR: ${REPO_ROOT}/Resources not found (vendor it into the submission)"; exit 1; }
[ -f "${REPO_ROOT}/Cargo.toml" ] || { echo "ERROR: ${REPO_ROOT}/Cargo.toml not found"; exit 1; }

gcloud config set project "${PROJECT_ID}" >/dev/null

echo "==> Enabling required APIs (idempotent)…"
gcloud services enable \
  run.googleapis.com \
  cloudbuild.googleapis.com \
  artifactregistry.googleapis.com \
  secretmanager.googleapis.com \
  --project "${PROJECT_ID}"

echo "==> Ensuring Artifact Registry repo '${REPO}' in ${REGION}…"
if ! gcloud artifacts repositories describe "${REPO}" \
      --location="${REGION}" --project="${PROJECT_ID}" >/dev/null 2>&1; then
  gcloud artifacts repositories create "${REPO}" \
    --repository-format=docker \
    --location="${REGION}" \
    --description="Container images for ${SERVICE}" \
    --project="${PROJECT_ID}"
else
  echo "    repo already exists — skipping."
fi

# --- Secret Manager ------------------------------------------------------
# create_or_update_secret NAME VALUE
create_or_update_secret() {
  local name="$1" value="$2"
  if ! gcloud secrets describe "${name}" --project="${PROJECT_ID}" >/dev/null 2>&1; then
    gcloud secrets create "${name}" --replication-policy=automatic --project="${PROJECT_ID}"
  fi
  printf '%s' "${value}" | gcloud secrets versions add "${name}" \
    --data-file=- --project="${PROJECT_ID}" >/dev/null
  echo "    secret '${name}' updated."
}

echo "==> Syncing secrets (only those provided via env)…"
if [ -n "${DATABASE_URL:-}" ]; then create_or_update_secret dmbr-database-url "${DATABASE_URL}"; fi
if [ -n "${SESSION_SECRET:-}" ]; then
  [ "${#SESSION_SECRET}" -ge 32 ] || { echo "ERROR: SESSION_SECRET must be >= 32 chars"; exit 1; }
  create_or_update_secret dmbr-session-secret "${SESSION_SECRET}"
fi
if [ -n "${ADMIN_USER:-}" ]; then create_or_update_secret dmbr-admin-user "${ADMIN_USER}"; fi
if [ -n "${ADMIN_PASSWORD:-}" ]; then create_or_update_secret dmbr-admin-password "${ADMIN_PASSWORD}"; fi

# Fail early if a required secret is missing entirely (neither env nor existing).
for s in dmbr-database-url dmbr-session-secret; do
  gcloud secrets describe "${s}" --project="${PROJECT_ID}" >/dev/null 2>&1 || {
    echo "ERROR: secret '${s}' does not exist and was not provided via env."; exit 1; }
done

# --- IAM for the Cloud Build service account -----------------------------
PROJECT_NUMBER="$(gcloud projects describe "${PROJECT_ID}" --format='value(projectNumber)')"
CB_SA="${PROJECT_NUMBER}@cloudbuild.gserviceaccount.com"
RUNTIME_SA="${PROJECT_NUMBER}-compute@developer.gserviceaccount.com"

echo "==> Granting Cloud Build SA (${CB_SA}) the deploy roles…"
for role in roles/run.admin roles/artifactregistry.writer \
            roles/secretmanager.secretAccessor roles/iam.serviceAccountUser; do
  gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${CB_SA}" --role="${role}" \
    --condition=None --quiet >/dev/null
done

echo "==> Granting the Cloud Run runtime SA access to the secrets…"
for s in dmbr-database-url dmbr-session-secret dmbr-admin-user dmbr-admin-password; do
  gcloud secrets describe "${s}" --project="${PROJECT_ID}" >/dev/null 2>&1 && \
  gcloud secrets add-iam-policy-binding "${s}" \
    --member="serviceAccount:${RUNTIME_SA}" \
    --role=roles/secretmanager.secretAccessor \
    --project="${PROJECT_ID}" --quiet >/dev/null || true
done

# --- Build + deploy ------------------------------------------------------
echo "==> Submitting Cloud Build…"
( cd "${REPO_ROOT}" && gcloud builds submit . \
    --config deploy/cloudbuild.yaml \
    --substitutions=_REGION="${REGION}",_REPO="${REPO}",_SERVICE="${SERVICE}" \
    --project "${PROJECT_ID}" )

URL="$(gcloud run services describe "${SERVICE}" --region="${REGION}" \
        --project="${PROJECT_ID}" --format='value(status.url)' 2>/dev/null || true)"
echo
echo "==> Done. Service URL: ${URL:-<deploy step in progress>}"
echo "    Admin console:    ${URL}/admin"
