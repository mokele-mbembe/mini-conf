#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load optional machine-local overrides first.
# shellcheck source=/dev/null
source "${script_dir}/load-dev-env.sh"

mini_conf_url_encode() {
  python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$1"
}

if [[ -z "${DATABASE_URL:-}" ]]; then
  db_scheme="${MINI_CONF_DB_SCHEME:-postgres}"
  db_host="${MINI_CONF_DB_HOST:-127.0.0.1}"
  db_port="${MINI_CONF_DB_PORT:-5432}"
  db_name="${MINI_CONF_DB_NAME:-mini_conf}"
  db_user="${MINI_CONF_DB_USER:-mini_conf}"
  secret_env="${MINI_CONF_SECRET_ENV:-dev}"

  if ! command -v secret-tool >/dev/null 2>&1; then
    echo "DATABASE_URL is not set and secret-tool is unavailable" >&2
    exit 1
  fi

  raw_password="$(secret-tool lookup service mini-conf env "${secret_env}" role app-db user "${db_user}")"

  if [[ -z "${raw_password}" ]]; then
    echo "secret-tool returned an empty password for mini-conf dev database" >&2
    exit 1
  fi

  encoded_password="$(mini_conf_url_encode "${raw_password}")"
  export DATABASE_URL="${db_scheme}://${db_user}:${encoded_password}@${db_host}:${db_port}/${db_name}"
fi

if [[ -z "${TEST_DATABASE_URL:-}" ]]; then
  export TEST_DATABASE_URL="${DATABASE_URL}"
fi

if [[ -z "${INIT_DB_ON_BOOT:-}" ]]; then
  export INIT_DB_ON_BOOT=true
fi
