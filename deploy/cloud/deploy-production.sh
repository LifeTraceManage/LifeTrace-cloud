#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/../.." && pwd)"
COMPOSE_FILE="${SCRIPT_DIR}/docker-compose.production.yml"
ENV_FILE="${SCRIPT_DIR}/.env.production"
COMPOSE_ENV_FILE="${SCRIPT_DIR}/.env"

fail() {
  printf '[LifeTrace deploy] ERROR: %s\n' "$*" >&2
  exit 1
}

command -v git >/dev/null 2>&1 || fail "git is required"
command -v docker >/dev/null 2>&1 || fail "docker is required"
docker compose version >/dev/null 2>&1 || fail "Docker Compose v2 is required"

[[ -f "${ENV_FILE}" ]] || fail "missing ${ENV_FILE}"
[[ -f "${COMPOSE_ENV_FILE}" ]] || fail "missing ${COMPOSE_ENV_FILE}"

if [[ "${1:-}" != "--skip-git-update" ]]; then
  [[ -z "$(git -C "${REPO_ROOT}" status --porcelain)" ]] || fail "repository has local changes"
  git -C "${REPO_ROOT}" fetch origin main
  git -C "${REPO_ROOT}" switch main
  git -C "${REPO_ROOT}" pull --ff-only origin main
fi

cd "${SCRIPT_DIR}"

docker compose --env-file "${COMPOSE_ENV_FILE}" -f "${COMPOSE_FILE}" config --quiet
docker compose --env-file "${COMPOSE_ENV_FILE}" -f "${COMPOSE_FILE}" pull
docker compose --env-file "${COMPOSE_ENV_FILE}" -f "${COMPOSE_FILE}" up -d --remove-orphans --wait

printf '[LifeTrace deploy] ready: %s\n' "${PUBLIC_WEB_BASE_URL:-http://8.148.75.45}"
printf '[LifeTrace deploy] BeeCount: %s\n' "${BEECOUNT_PUBLIC_BASE_URL:-http://8.148.75.45:8869}"
