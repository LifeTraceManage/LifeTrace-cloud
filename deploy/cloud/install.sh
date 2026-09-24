#!/usr/bin/env bash
set -Eeuo pipefail

REPO_URL="https://github.com/LifeTraceManage/LifeTrace-cloud.git"
REF="${LIFETRACE_REF:-main}"
INSTALL_DIR="${LIFETRACE_INSTALL_DIR:-$HOME/lifetrace}"
BASE_URL="${PUBLIC_WEB_BASE_URL:-}"
SKIP_VERIFY="false"

usage() {
  cat <<'EOF'
LifeTrace local-build installer

Usage:
  install.sh --base-url http://YOUR_SERVER_IP [options]

Options:
  --base-url URL   Public URL used by LifeTrace auth/CORS (required on first install)
  --dir PATH       Install directory (default: ~/lifetrace)
  --ref REF        Git branch/tag/commit to deploy (default: main)
  --skip-verify    Build and start services without running verify-production.sh
  --tag TAG        Deprecated compatibility option; ignored in local-build mode
  -h, --help       Show this help

Environment equivalents:
  PUBLIC_WEB_BASE_URL, LIFETRACE_INSTALL_DIR, LIFETRACE_REF
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
    --ref)
      [[ $# -ge 2 ]] || { echo "[LifeTrace install] --ref requires a value" >&2; exit 2; }
      REF="$2"; shift 2 ;;
    --tag)
      [[ $# -ge 2 ]] || { echo "[LifeTrace install] --tag requires a value" >&2; exit 2; }
      echo "[LifeTrace install] --tag is ignored; production images are built locally."
      shift 2 ;;
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

command -v git >/dev/null 2>&1 || {
  echo "[LifeTrace install] git is required" >&2
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
SOURCE_DIR="$INSTALL_DIR/source"

if [[ -d "$SOURCE_DIR/.git" ]]; then
  echo "[LifeTrace install] updating source: $REF"
  git -C "$SOURCE_DIR" fetch --depth 1 origin "$REF"
  git -C "$SOURCE_DIR" checkout --detach FETCH_HEAD
  git -C "$SOURCE_DIR" reset --hard FETCH_HEAD
else
  if [[ -e "$SOURCE_DIR" ]]; then
    echo "[LifeTrace install] $SOURCE_DIR exists but is not a git repository" >&2
    exit 1
  fi
  echo "[LifeTrace install] cloning source: $REF"
  git clone --depth 1 --branch "$REF" "$REPO_URL" "$SOURCE_DIR"
fi

SCRIPT_DIR="$SOURCE_DIR/deploy/cloud"
ENV_FILE="$SCRIPT_DIR/.env.production"
LEGACY_ENV_FILE="$INSTALL_DIR/.env.production"
DEPLOY_FILE="$SCRIPT_DIR/deploy-production.sh"
VERIFY_FILE="$SCRIPT_DIR/verify-production.sh"

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

if [[ ! -f "$ENV_FILE" && -f "$LEGACY_ENV_FILE" ]]; then
  cp "$LEGACY_ENV_FILE" "$ENV_FILE"
  echo "[LifeTrace install] migrated existing environment from $LEGACY_ENV_FILE"
fi

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

LIFETRACE_WEB_IMAGE=lifetrace-web-app:local
LIFETRACE_CLOUD_IMAGE=lifetrace-cloud:local

# Optional Mail aggregation:
# MAIL_CREDENTIAL_KEY=
EOF
  echo "[LifeTrace install] created $ENV_FILE with generated secrets"
else
  echo "[LifeTrace install] preserving existing secrets in $ENV_FILE"
fi

set_env_value() {
  local key="$1"
  local value="$2"
  if grep -q "^$key=" "$ENV_FILE"; then
    sed -i "s|^$key=.*|$key=$value|" "$ENV_FILE"
  else
    printf '%s=%s\n' "$key" "$value" >>"$ENV_FILE"
  fi
}

# Local-build mode must never depend on GHCR at runtime.
set_env_value "LIFETRACE_WEB_IMAGE" "lifetrace-web-app:local"
set_env_value "LIFETRACE_CLOUD_IMAGE" "lifetrace-cloud:local"

if [[ -n "$BASE_URL" ]]; then
  set_env_value "PUBLIC_WEB_BASE_URL" "$BASE_URL"
  set_env_value "CORS_ALLOWED_ORIGINS" "$BASE_URL"
  if [[ "$BASE_URL" == https://* ]]; then
    set_env_value "AUTH_COOKIE_SECURE" "true"
  else
    set_env_value "AUTH_COOKIE_SECURE" "false"
  fi
fi

chmod 600 "$ENV_FILE"
chmod +x "$DEPLOY_FILE" "$VERIFY_FILE"

echo "[LifeTrace install] source directory: $SOURCE_DIR"
echo "[LifeTrace install] building Web and Cloud locally"
"$DEPLOY_FILE" all

if [[ "$SKIP_VERIFY" != "true" ]]; then
  "$VERIFY_FILE"
fi

echo
echo "[LifeTrace install] deployment complete"
echo "  URL: $(grep '^PUBLIC_WEB_BASE_URL=' "$ENV_FILE" | cut -d= -f2-)"
echo "  Source: $SOURCE_DIR"
echo "  Data: Docker volume lifetrace_lifetrace_data"
