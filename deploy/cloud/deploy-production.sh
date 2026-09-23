#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="${SCRIPT_DIR}/.env.production"
COMPOSE_FILE="${SCRIPT_DIR}/docker-compose.production.yml"
WEB_SOURCE_DIR="${LIFETRACE_WEB_SOURCE_DIR:-${SCRIPT_DIR}/../../../LifeTrace-web}"
WEB_DIST_DIR="${SCRIPT_DIR}/.web-dist"

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
[[ -f "${WEB_SOURCE_DIR}/package.json" && -f "${WEB_SOURCE_DIR}/package-lock.json" ]] || {
  echo "[LifeTrace deploy] LifeTrace-web checkout not found at ${WEB_SOURCE_DIR}" >&2
  echo "[LifeTrace deploy] set LIFETRACE_WEB_SOURCE_DIR=/path/to/LifeTrace-web if it is not a sibling checkout" >&2
  exit 1
}
WEB_SOURCE_DIR="$(cd -- "${WEB_SOURCE_DIR}" && pwd)"

echo "[LifeTrace deploy] building LifeTrace-web from ${WEB_SOURCE_DIR}"
rm -rf "${WEB_DIST_DIR}"
mkdir -p "${WEB_DIST_DIR}"

docker run --rm \
  --user "$(id -u):$(id -g)" \
  -e HOME=/tmp \
  -v "${WEB_SOURCE_DIR}:/src:ro" \
  -v "${WEB_DIST_DIR}:/out" \
  node:22-alpine \
  sh -ec '
    mkdir -p /tmp/web
    cp -R /src/. /tmp/web/
    cd /tmp/web
    npm ci --no-audit --no-fund
    npm run build
    test -s dist/index.html
    cp -R dist/. /out/
  '

[[ -s "${WEB_DIST_DIR}/index.html" ]] || {
  echo "[LifeTrace deploy] LifeTrace-web build did not produce ${WEB_DIST_DIR}/index.html" >&2
  exit 1
}

cd "${SCRIPT_DIR}"
compose=(docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}")
"${compose[@]}" pull
"${compose[@]}" up -d --remove-orphans --wait
"${compose[@]}" ps
