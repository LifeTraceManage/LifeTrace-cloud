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

read_env_value() {
  local key="$1"
  sed -n "s/^${key}=//p" "${ENV_FILE}" | tail -n 1 | tr -d '\r'
}

domain="$(read_env_value LIFETRACE_DOMAIN)"
public_ip="$(read_env_value LIFETRACE_PUBLIC_IP)"
if [[ -z "${domain}" ]]; then
  echo "[LifeTrace deploy] LIFETRACE_DOMAIN is required in ${ENV_FILE}" >&2
  exit 1
fi
if [[ -z "${public_ip}" ]]; then
  echo "[LifeTrace deploy] LIFETRACE_PUBLIC_IP is required in ${ENV_FILE}" >&2
  exit 1
fi

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

caddy_image="$(read_env_value LIFETRACE_CADDY_IMAGE)"
caddy_image="${caddy_image:-caddy:2-alpine}"
if ! docker image inspect "${caddy_image}" >/dev/null 2>&1; then
  echo "[LifeTrace deploy] pulling gateway image: ${caddy_image}"
  docker pull "${caddy_image}"
fi

echo "[LifeTrace deploy] validating deployment"
"${compose[@]}" config >/dev/null

echo "[LifeTrace deploy] building local image(s): ${build_services[*]}"
"${compose[@]}" build "${build_services[@]}"

echo "[LifeTrace deploy] starting LifeTrace"
"${compose[@]}" up -d --remove-orphans --wait --pull never
"${compose[@]}" ps

echo "[LifeTrace deploy] IP fallback: http://${public_ip}"
echo "[LifeTrace deploy] HTTPS endpoint (after DNS/ICP is available): https://${domain}"
echo "[LifeTrace deploy] gateway logs: docker compose --env-file .env.production -f docker-compose.production.yml logs caddy"
