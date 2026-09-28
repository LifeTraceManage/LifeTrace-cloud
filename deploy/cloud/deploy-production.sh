#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="${SCRIPT_DIR}/.env.production"
ENV_EXAMPLE_FILE="${SCRIPT_DIR}/.env.production.example"
COMPOSE_FILE="${SCRIPT_DIR}/docker-compose.production.yml"
TARGET="${1:-all}"

read_env_value() {
  local key="$1"
  sed -n "s/^${key}=//p" "${ENV_FILE}" | tail -n 1 | tr -d '\r'
}

init_env() {
  [[ -f "${ENV_EXAMPLE_FILE}" ]] || {
    echo "[LifeTrace deploy] missing ${ENV_EXAMPLE_FILE}" >&2
    exit 1
  }

  if [[ ! -f "${ENV_FILE}" ]]; then
    cp "${ENV_EXAMPLE_FILE}" "${ENV_FILE}"
    chmod 600 "${ENV_FILE}"
    echo "[LifeTrace deploy] created ${ENV_FILE} from .env.production.example"
    echo "[LifeTrace deploy] review placeholder secrets and endpoint values before deployment"
    return 0
  fi

  local added=0
  local line key
  local -a added_keys=()

  while IFS= read -r line || [[ -n "${line}" ]]; do
    line="${line%$'\r'}"
    [[ "${line}" == *=* ]] || continue

    key="${line%%=*}"
    [[ "${key}" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue

    if ! grep -qE "^${key}=" "${ENV_FILE}"; then
      if (( added == 0 )); then
        printf '\n# Added by deploy-production.sh init-env\n' >> "${ENV_FILE}"
      fi
      printf '%s\n' "${line}" >> "${ENV_FILE}"
      added_keys+=("${key}")
      ((added += 1))
    fi
  done < "${ENV_EXAMPLE_FILE}"

  chmod 600 "${ENV_FILE}"

  if (( added == 0 )); then
    echo "[LifeTrace deploy] ${ENV_FILE} is already up to date"
  else
    echo "[LifeTrace deploy] added ${added} missing variable(s) to ${ENV_FILE}:"
    printf '  - %s\n' "${added_keys[@]}"
    echo "[LifeTrace deploy] existing values and secrets were preserved"
  fi
}

if [[ "${TARGET}" == "init-env" ]]; then
  init_env
  exit 0
fi

case "${TARGET}" in
  all) build_services=(cloud web) ;;
  cloud) build_services=(cloud) ;;
  web) build_services=(web) ;;
  *)
    echo "[LifeTrace deploy] usage: $0 [init-env|all|cloud|web]" >&2
    exit 2
    ;;
esac

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
  echo "[LifeTrace deploy] run: ./deploy-production.sh init-env" >&2
  exit 1
}

domain="$(read_env_value LIFETRACE_DOMAIN)"
public_ip="$(read_env_value LIFETRACE_PUBLIC_IP)"
if [[ -z "${domain}" ]]; then
  echo "[LifeTrace deploy] LIFETRACE_DOMAIN is required in ${ENV_FILE}" >&2
  echo "[LifeTrace deploy] run: ./deploy-production.sh init-env" >&2
  exit 1
fi
if [[ -z "${public_ip}" ]]; then
  echo "[LifeTrace deploy] LIFETRACE_PUBLIC_IP is required in ${ENV_FILE}" >&2
  echo "[LifeTrace deploy] run: ./deploy-production.sh init-env" >&2
  exit 1
fi

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
