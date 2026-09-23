#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="${SCRIPT_DIR}/.env.production"
COMPOSE_FILE="${SCRIPT_DIR}/docker-compose.production.yml"

command -v docker >/dev/null 2>&1 || {
  echo "[LifeTrace deploy] docker is required" >&2
  exit 1
}
docker compose version >/dev/null 2>&1 || {
  echo "[LifeTrace deploy] Docker Compose v2 is required" >&2
  exit 1
}
[[ -f "${ENV_FILE}" ]] || {
  echo "[LifeTrace deploy] missing ${ENV_FILE}" >&2
  exit 1
}

cd "${SCRIPT_DIR}"
compose=(docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}")

echo "[LifeTrace deploy] pulling prebuilt LifeTrace image"
if ! "${compose[@]}" pull lifetrace; then
  echo "[LifeTrace deploy] image pull failed." >&2
  echo "[LifeTrace deploy] if the GHCR package is private, login first:" >&2
  echo "  echo <GITHUB_TOKEN> | docker login ghcr.io -u <GITHUB_USER> --password-stdin" >&2
  exit 1
fi

"${compose[@]}" up -d --remove-orphans --wait
"${compose[@]}" ps
