#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="${SCRIPT_DIR}/.env.production"
COMPOSE_FILE="${SCRIPT_DIR}/docker-compose.production.yml"
TARGET="${1:-all}"

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

case "${TARGET}" in
  all) build_services=(cloud web) ;;
  cloud) build_services=(cloud) ;;
  web) build_services=(web) ;;
  *)
    echo "[LifeTrace deploy] usage: $0 [all|cloud|web]" >&2
    exit 2
    ;;
esac

cd "${SCRIPT_DIR}"
compose=(docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}")

echo "[LifeTrace deploy] building local image(s): ${build_services[*]}"
"${compose[@]}" build "${build_services[@]}"

echo "[LifeTrace deploy] starting LifeTrace with local images only"
"${compose[@]}" up -d --remove-orphans --wait --pull never
"${compose[@]}" ps
