#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="${SCRIPT_DIR}/.env.production"
COMPOSE_FILE="${SCRIPT_DIR}/docker-compose.production.yml"
WEB_DIST_DIR="${SCRIPT_DIR}/.web-dist"

command -v docker >/dev/null 2>&1 || {
  echo "[LifeTrace verify] docker is required" >&2
  exit 1
}
docker compose version >/dev/null 2>&1 || {
  echo "[LifeTrace verify] Docker Compose v2 is required" >&2
  exit 1
}
[[ -f "${ENV_FILE}" ]] || {
  echo "[LifeTrace verify] missing ${ENV_FILE}" >&2
  exit 1
}
[[ -s "${WEB_DIST_DIR}/index.html" ]] || {
  echo "[LifeTrace verify] missing built Web entrypoint ${WEB_DIST_DIR}/index.html" >&2
  echo "[LifeTrace verify] run deploy-production.sh first" >&2
  exit 1
}

cd "${SCRIPT_DIR}"
compose=(docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}")

"${compose[@]}" config --quiet

service_count="$("${compose[@]}" config --services | wc -l | tr -d ' ')"
[[ "${service_count}" == "1" ]] || {
  echo "[LifeTrace verify] expected exactly one service, got ${service_count}" >&2
  exit 1
}

"${compose[@]}" up -d --remove-orphans --wait

"${compose[@]}" exec -T lifetrace sh -ec 'test -s /app/web/index.html'
"${compose[@]}" exec -T lifetrace curl --fail --silent http://127.0.0.1:8787/ >/dev/null
"${compose[@]}" exec -T lifetrace curl --fail --silent http://127.0.0.1:8787/health/ready >/dev/null
"${compose[@]}" exec -T lifetrace curl --fail --silent http://127.0.0.1:8869/ready >/dev/null

before="$("${compose[@]}" exec -T lifetrace sh -ec 'test -s /data/lifetrace.db && stat -c "%i:%s" /data/lifetrace.db')"

"${compose[@]}" restart lifetrace
"${compose[@]}" up -d --wait

"${compose[@]}" exec -T lifetrace sh -ec 'test -s /app/web/index.html'
"${compose[@]}" exec -T lifetrace curl --fail --silent http://127.0.0.1:8787/ >/dev/null
"${compose[@]}" exec -T lifetrace curl --fail --silent http://127.0.0.1:8787/health/ready >/dev/null
"${compose[@]}" exec -T lifetrace curl --fail --silent http://127.0.0.1:8869/ready >/dev/null

after="$("${compose[@]}" exec -T lifetrace sh -ec 'test -s /data/lifetrace.db && stat -c "%i:%s" /data/lifetrace.db')"

before_inode="${before%%:*}"
after_inode="${after%%:*}"
[[ "${before_inode}" == "${after_inode}" ]] || {
  echo "[LifeTrace verify] SQLite file was not preserved across restart" >&2
  exit 1
}

echo "[LifeTrace verify] OK"
echo "  service: lifetrace"
echo "  web:     /app/web <- deploy/cloud/.web-dist"
echo "  storage: /data/lifetrace.db"
echo "  api:     http://127.0.0.1:8787/health/ready"
echo "  beecount:http://127.0.0.1:8869/ready"
