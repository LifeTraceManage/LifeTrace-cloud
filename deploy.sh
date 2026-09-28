#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="${ROOT_DIR}/deploy/cloud"
DEPLOY_SCRIPT="${DEPLOY_DIR}/deploy-production.sh"
VERIFY_SCRIPT="${DEPLOY_DIR}/verify-production.sh"
ENV_FILE="${DEPLOY_DIR}/.env.production"
TARGET="${1:-all}"
REF="${LIFETRACE_DEPLOY_REF:-main}"
SKIP_UPDATE="${LIFETRACE_SKIP_UPDATE:-false}"
SKIP_VERIFY="${LIFETRACE_SKIP_VERIFY:-false}"

usage() {
  cat <<'EOF'
LifeTrace one-click deployment

Usage:
  ./deploy.sh [all|web|cloud]

Default:
  ./deploy.sh        # update main, sync env, deploy all, verify

Environment:
  LIFETRACE_DEPLOY_REF=<branch|tag>  Git ref to deploy (default: main)
  LIFETRACE_SKIP_UPDATE=true         Skip git fetch/pull
  LIFETRACE_SKIP_VERIFY=true         Skip post-deploy verification
EOF
}

case "${TARGET}" in
  all|web|cloud) ;;
  -h|--help)
    usage
    exit 0
    ;;
  *)
    echo "[LifeTrace] usage: ./deploy.sh [all|web|cloud]" >&2
    exit 2
    ;;
esac

[[ -x "${DEPLOY_SCRIPT}" || -f "${DEPLOY_SCRIPT}" ]] || {
  echo "[LifeTrace] missing deployment script: ${DEPLOY_SCRIPT}" >&2
  exit 1
}

command -v git >/dev/null 2>&1 || {
  echo "[LifeTrace] git is required" >&2
  exit 1
}
command -v docker >/dev/null 2>&1 || {
  echo "[LifeTrace] Docker Engine is required" >&2
  exit 1
}
docker compose version >/dev/null 2>&1 || {
  echo "[LifeTrace] Docker Compose v2 is required" >&2
  exit 1
}
docker info >/dev/null 2>&1 || {
  echo "[LifeTrace] current user cannot access the Docker daemon" >&2
  exit 1
}

echo "[LifeTrace] repository: ${ROOT_DIR}"
echo "[LifeTrace] target:     ${TARGET}"

if [[ "${SKIP_UPDATE}" != "true" ]]; then
  if [[ -n "$(git -C "${ROOT_DIR}" status --porcelain --untracked-files=no)" ]]; then
    echo "[LifeTrace] tracked files have local changes; refusing to overwrite them." >&2
    echo "[LifeTrace] commit/stash the changes, or deploy current code with:" >&2
    echo "  LIFETRACE_SKIP_UPDATE=true ./deploy.sh ${TARGET}" >&2
    exit 1
  fi

  echo "[LifeTrace] updating source from origin/${REF}"
  git -C "${ROOT_DIR}" fetch origin "${REF}"

  if git -C "${ROOT_DIR}" show-ref --verify --quiet "refs/heads/${REF}"; then
    git -C "${ROOT_DIR}" checkout "${REF}" >/dev/null
    git -C "${ROOT_DIR}" merge --ff-only FETCH_HEAD
  else
    git -C "${ROOT_DIR}" checkout -B "${REF}" FETCH_HEAD >/dev/null
  fi

  echo "[LifeTrace] source updated; reloading deployment script"
  exec env \
    LIFETRACE_SKIP_UPDATE=true \
    LIFETRACE_SKIP_VERIFY="${SKIP_VERIFY}" \
    LIFETRACE_DEPLOY_REF="${REF}" \
    bash "${ROOT_DIR}/deploy.sh" "${TARGET}"
else
  echo "[LifeTrace] source update skipped"
fi

chmod +x "${DEPLOY_SCRIPT}"
[[ -f "${VERIFY_SCRIPT}" ]] && chmod +x "${VERIFY_SCRIPT}"

echo "[LifeTrace] synchronizing production environment"
"${DEPLOY_SCRIPT}" init-env

[[ -f "${ENV_FILE}" ]] || {
  echo "[LifeTrace] missing ${ENV_FILE} after init-env" >&2
  exit 1
}

placeholder_keys=()
for key in CURSOR_SIGNING_KEY PAGE_TOKEN_SIGNING_KEY AUTH_PASSWORD_PEPPER AUTH_TOKEN_HASH_PEPPER; do
  value="$(sed -n "s/^${key}=//p" "${ENV_FILE}" | tail -n 1 | tr -d '\r')"
  if [[ -z "${value}" || "${value}" == replace-with-* ]]; then
    placeholder_keys+=("${key}")
  fi
done

if (( ${#placeholder_keys[@]} > 0 )); then
  echo "[LifeTrace] production secrets still contain template placeholders:" >&2
  printf '  - %s\n' "${placeholder_keys[@]}" >&2
  echo "[LifeTrace] configure real secrets in ${ENV_FILE} before deployment." >&2
  exit 1
fi

echo "[LifeTrace] deploying ${TARGET}"
"${DEPLOY_SCRIPT}" "${TARGET}"

if [[ "${SKIP_VERIFY}" != "true" ]]; then
  echo "[LifeTrace] verifying production services"
  "${VERIFY_SCRIPT}"
else
  echo "[LifeTrace] verification skipped"
fi

public_url="$(sed -n 's/^PUBLIC_WEB_BASE_URL=//p' "${ENV_FILE}" | tail -n 1 | tr -d '\r')"
public_ip="$(sed -n 's/^LIFETRACE_PUBLIC_IP=//p' "${ENV_FILE}" | tail -n 1 | tr -d '\r')"

echo
echo "[LifeTrace] deployment complete"
[[ -n "${public_url}" ]] && echo "  Web: ${public_url}"
[[ -n "${public_ip}" ]] && echo "  IP fallback: http://${public_ip}"
echo "  Commit: $(git -C "${ROOT_DIR}" rev-parse --short HEAD)"
