#!/usr/bin/env bash
set -Eeuo pipefail

REPO_RAW_BASE="https://raw.githubusercontent.com/LifeTraceManage/LifeTrace-cloud"
REF="${LIFETRACE_REF:-main}"
INSTALL_DIR="${LIFETRACE_INSTALL_DIR:-$HOME/lifetrace}"
BASE_URL="${PUBLIC_WEB_BASE_URL:-}"
IMAGE_TAG="${LIFETRACE_IMAGE_TAG:-main}"
SKIP_VERIFY="false"

usage() {
  cat <<'EOF'
LifeTrace one-command installer

Usage:
  install.sh --base-url http://YOUR_SERVER_IP [options]

Options:
  --base-url URL   Public URL used by LifeTrace auth/CORS (required on first install)
  --dir PATH       Install directory (default: ~/lifetrace)
  --tag TAG        Image tag for both Web and Cloud (default: main; e.g. sha-abcdef0)
  --ref REF        Git ref used to download deployment files (default: main)
  --skip-verify    Start services without running verify-production.sh
  -h, --help       Show this help

Environment equivalents:
  PUBLIC_WEB_BASE_URL, LIFETRACE_INSTALL_DIR, LIFETRACE_IMAGE_TAG, LIFETRACE_REF
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-url)
      [[ $# -ge 2 ]] || { echo "[LifeTrace install] --base-url requires a value" >&2; exit 2; }
      BASE_URL="$2"; shift 2 ;;
    --dir)
      [[ $# -ge 2 ]] || { echo "[LifeTrace install] --dir requires a value" >&2; exit 2; }
      INSTALL_DIR="$2"; shift 2 ;;
    --tag)
      [[ $# -ge 2 ]] || { echo "[LifeTrace install] --tag requires a value" >&2; exit 2; }
      IMAGE_TAG="$2"; shift 2 ;;
    --ref)
      [[ $# -ge 2 ]] || { echo "[LifeTrace install] --ref requires a value" >&2; exit 2; }
      REF="$2"; shift 2 ;;
    --skip-verify)
      SKIP_VERIFY="true"; shift ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "[LifeTrace install] unknown argument: $1" >&2
      usage >&2
      exit 2 ;;
  esac
done

command -v curl >/dev/null 2>&1 || {
  echo "[LifeTrace install] curl is required" >&2
  exit 1
}
command -v docker >/dev/null 2>&1 || {
  echo "[LifeTrace install] Docker Engine is required" >&2
  exit 1
}
docker compose version >/dev/null 2>&1 || {
  echo "[LifeTrace install] Docker Compose v2 is required" >&2
  exit 1
}

if ! docker info >/dev/null 2>&1; then
  echo "[LifeTrace install] current user cannot access the Docker daemon." >&2
  echo "[LifeTrace install] configure Docker permissions or run this installer as a user with Docker access." >&2
  exit 1
fi

if [[ -n "$BASE_URL" && ! "$BASE_URL" =~ ^https?://[^[:space:]]+$ ]]; then
  echo "[LifeTrace install] --base-url must start with http:// or https://" >&2
  exit 2
fi

mkdir -p "$INSTALL_DIR"
INSTALL_DIR="$(cd -- "$INSTALL_DIR" && pwd)"
ENV_FILE="$INSTALL_DIR/.env.production"
COMPOSE_FILE="$INSTALL_DIR/docker-compose.production.yml"
DEPLOY_FILE="$INSTALL_DIR/deploy-production.sh"
VERIFY_FILE="$INSTALL_DIR/verify-production.sh"

download() {
  local path="$1"
  local target="$2"
  local url="$REPO_RAW_BASE/$REF/deploy/cloud/$path"
  echo "[LifeTrace install] downloading $path"
  curl --fail --silent --show-error --location "$url" --output "$target.tmp"
  mv "$target.tmp" "$target"
}

download "docker-compose.production.yml" "$COMPOSE_FILE"
download "deploy-production.sh" "$DEPLOY_FILE"
download "verify-production.sh" "$VERIFY_FILE"
chmod +x "$DEPLOY_FILE" "$VERIFY_FILE"

random_secret() {
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -hex 32
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c "import secrets; print(secrets.token_hex(32))"
  else
    echo "[LifeTrace install] openssl or python3 is required to generate secrets" >&2
    return 1
  fi
}

if [[ ! -f "$ENV_FILE" ]]; then
  [[ -n "$BASE_URL" ]] || {
    echo "[LifeTrace install] --base-url is required on first install." >&2
    echo "Example: --base-url http://203.0.113.10" >&2
    exit 2
  }

  cookie_secure="false"
  [[ "$BASE_URL" == https://* ]] && cookie_secure="true"

  umask 077
  cat >"$ENV_FILE" <<EOF
CURSOR_SIGNING_KEY=$(random_secret)
PAGE_TOKEN_SIGNING_KEY=$(random_secret)
AUTH_PASSWORD_PEPPER=$(random_secret)
AUTH_TOKEN_HASH_PEPPER=$(random_secret)

PUBLIC_WEB_BASE_URL=$BASE_URL
CORS_ALLOWED_ORIGINS=$BASE_URL
AUTH_COOKIE_SECURE=$cookie_secure

LIFETRACE_WEB_IMAGE=ghcr.io/lifetracemanage/lifetrace-web-app:$IMAGE_TAG
LIFETRACE_CLOUD_IMAGE=ghcr.io/lifetracemanage/lifetrace-cloud:$IMAGE_TAG

# Optional Mail aggregation:
# MAIL_CREDENTIAL_KEY=
EOF
  echo "[LifeTrace install] created $ENV_FILE with generated secrets"
else
  echo "[LifeTrace install] preserving existing $ENV_FILE"
  if [[ "$IMAGE_TAG" != "main" ]]; then
    echo "[LifeTrace install] note: existing env controls image tags; --tag is only written on first install."
  fi
fi

echo "[LifeTrace install] install directory: $INSTALL_DIR"
cd "$INSTALL_DIR"

if ! "$DEPLOY_FILE"; then
  echo >&2
  echo "[LifeTrace install] deployment failed." >&2
  echo "[LifeTrace install] if GHCR images are private, authenticate and retry:" >&2
  echo "  echo <GITHUB_TOKEN> | docker login ghcr.io -u <GITHUB_USER> --password-stdin" >&2
  exit 1
fi

if [[ "$SKIP_VERIFY" != "true" ]]; then
  "$VERIFY_FILE"
fi

echo
echo "[LifeTrace install] deployment complete"
echo "  URL: ${BASE_URL:-$(grep '^PUBLIC_WEB_BASE_URL=' "$ENV_FILE" | cut -d= -f2-)}"
echo "  Dir: $INSTALL_DIR"
echo "  Data: Docker volume lifetrace_lifetrace_data"
