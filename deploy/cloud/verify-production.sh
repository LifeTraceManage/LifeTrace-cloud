#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="${SCRIPT_DIR}/.env.production"
COMPOSE_FILE="${SCRIPT_DIR}/docker-compose.production.yml"

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

cd "${SCRIPT_DIR}"
compose=(docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}")

"${compose[@]}" config --quiet

service_count="$("${compose[@]}" config --services | wc -l | tr -d ' ')"
[[ "${service_count}" == "2" ]] || {
  echo "[LifeTrace verify] expected exactly two services, got ${service_count}" >&2
  exit 1
}

"${compose[@]}" up -d --remove-orphans --wait

# Cloud runtime and embedded SQLite.
"${compose[@]}" exec -T cloud sh -ec 'test -x /app/lifetrace-cloud'
"${compose[@]}" exec -T cloud curl --fail --silent http://127.0.0.1:8787/health/ready >/dev/null
"${compose[@]}" exec -T cloud curl --fail --silent http://127.0.0.1:8869/ready >/dev/null

# Web static payload and reverse proxy to Cloud.
"${compose[@]}" exec -T web sh -ec 'test -s /usr/share/nginx/html/index.html'
"${compose[@]}" exec -T web wget -q -O - http://127.0.0.1/healthz >/dev/null
"${compose[@]}" exec -T web wget -q -O - http://127.0.0.1/health/ready >/dev/null

before="$("${compose[@]}" exec -T cloud sh -ec 'test -s /data/lifetrace.db && stat -c "%i:%s" /data/lifetrace.db')"

"${compose[@]}" restart cloud
"${compose[@]}" up -d --wait cloud web

"${compose[@]}" exec -T cloud curl --fail --silent http://127.0.0.1:8787/health/ready >/dev/null
"${compose[@]}" exec -T web wget -q -O - http://127.0.0.1/health/ready >/dev/null

after="$("${compose[@]}" exec -T cloud sh -ec 'test -s /data/lifetrace.db && stat -c "%i:%s" /data/lifetrace.db')"

before_inode="${before%%:*}"
after_inode="${after%%:*}"
[[ "${before_inode}" == "${after_inode}" ]] || {
  echo "[LifeTrace verify] SQLite file was not preserved across Cloud restart" >&2
  exit 1
}

echo "[LifeTrace verify] OK"
echo "  web:      ghcr.io/lifetracemanage/lifetrace-web"
echo "  cloud:    ghcr.io/lifetracemanage/lifetrace-cloud"
echo "  storage:  /data/lifetrace.db"
echo "  public:   http://127.0.0.1/"
echo "  beecount: http://127.0.0.1:8869/ready"
