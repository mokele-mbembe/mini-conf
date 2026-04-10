#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=/dev/null
source "${script_dir}/load-dev-env.sh"

mini_conf_url_encode() {
  python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$1"
}

read_password_from_file() {
  local file_path="$1"

  if [[ ! -f "${file_path}" ]]; then
    echo "${file_path} does not exist" >&2
    exit 1
  fi

  <"${file_path}"
}

resolve_password() {
  local inline_password="$1"
  local password_file="$2"
  local secret_env="$3"
  local db_user="$4"

  if [[ -n "${inline_password}" ]]; then
    printf '%s' "${inline_password}"
    return 0
  fi

  if [[ -n "${password_file}" ]]; then
    read_password_from_file "${password_file}"
    return 0
  fi

  if command -v secret-tool >/dev/null 2>&1 && [[ -n "${db_user}" ]]; then
    secret-tool lookup service mini-conf env "${secret_env}" role app-db user "${db_user}"
    return 0
  fi

  printf '%s' ""
}

build_database_url() {
  local label="$1"
  local direct_url="$2"
  local db_scheme="$3"
  local db_host="$4"
  local db_port="$5"
  local db_name="$6"
  local db_user="$7"
  local db_password="$8"

  if [[ -n "${direct_url}" ]]; then
    printf '%s' "${direct_url}"
    return 0
  fi

  if [[ -z "${db_name}" || -z "${db_user}" ]]; then
    printf '%s' ""
    return 0
  fi

  if [[ -z "${db_password}" ]]; then
    echo "${label} password is empty; set an inline password, a password file, or a secret-tool entry" >&2
    exit 1
  fi

  local encoded_password
  encoded_password="$(mini_conf_url_encode "${db_password}")"
  printf '%s' "${db_scheme}://${db_user}:${encoded_password}@${db_host}:${db_port}/${db_name}"
}

if [[ -z "${DATABASE_URL:-}" ]]; then
  runtime_password=""

  if [[ -z "${MINI_CONF_LOCAL_DATABASE_URL:-}" ]]; then
    runtime_password="$(resolve_password \
      "${MINI_CONF_LOCAL_DB_PASSWORD:-${MINI_CONF_DB_PASSWORD:-}}" \
      "${MINI_CONF_LOCAL_DB_PASSWORD_FILE:-${MINI_CONF_DB_PASSWORD_FILE:-}}" \
      "${MINI_CONF_LOCAL_SECRET_ENV:-${MINI_CONF_SECRET_ENV:-dev}}" \
      "${MINI_CONF_LOCAL_DB_USER:-${MINI_CONF_DB_USER:-}}")"
  fi

  runtime_url="$(build_database_url \
    "runtime database" \
    "${MINI_CONF_LOCAL_DATABASE_URL:-}" \
    "${MINI_CONF_LOCAL_DB_SCHEME:-${MINI_CONF_DB_SCHEME:-postgres}}" \
    "${MINI_CONF_LOCAL_DB_HOST:-${MINI_CONF_DB_HOST:-127.0.0.1}}" \
    "${MINI_CONF_LOCAL_DB_PORT:-${MINI_CONF_DB_PORT:-5432}}" \
    "${MINI_CONF_LOCAL_DB_NAME:-${MINI_CONF_DB_NAME:-}}" \
    "${MINI_CONF_LOCAL_DB_USER:-${MINI_CONF_DB_USER:-}}" \
    "${runtime_password}")"

  if [[ -n "${runtime_url}" ]]; then
    export DATABASE_URL="${runtime_url}"
  fi
fi

if [[ -z "${TEST_DATABASE_URL:-}" ]]; then
  if [[ "${MINI_CONF_LOCAL_TEST_USE_RUNTIME_DB:-false}" == "true" ]] \
    && [[ -z "${MINI_CONF_LOCAL_TEST_DATABASE_URL:-}" ]] \
    && [[ -z "${MINI_CONF_LOCAL_TEST_DB_SCHEME:-}" ]] \
    && [[ -z "${MINI_CONF_LOCAL_TEST_DB_HOST:-}" ]] \
    && [[ -z "${MINI_CONF_LOCAL_TEST_DB_PORT:-}" ]] \
    && [[ -z "${MINI_CONF_LOCAL_TEST_DB_NAME:-}" ]] \
    && [[ -z "${MINI_CONF_LOCAL_TEST_DB_USER:-}" ]] \
    && [[ -z "${MINI_CONF_LOCAL_TEST_DB_PASSWORD:-}" ]] \
    && [[ -z "${MINI_CONF_LOCAL_TEST_DB_PASSWORD_FILE:-}" ]]; then
    if [[ -z "${DATABASE_URL:-}" ]]; then
      echo "MINI_CONF_LOCAL_TEST_USE_RUNTIME_DB=true requires DATABASE_URL or local runtime DB settings" >&2
      exit 1
    fi

    export TEST_DATABASE_URL="${DATABASE_URL}"
  else
    test_password=""

    if [[ -z "${MINI_CONF_LOCAL_TEST_DATABASE_URL:-}" ]]; then
      test_password="$(resolve_password \
        "${MINI_CONF_LOCAL_TEST_DB_PASSWORD:-}" \
        "${MINI_CONF_LOCAL_TEST_DB_PASSWORD_FILE:-}" \
        "${MINI_CONF_LOCAL_TEST_SECRET_ENV:-${MINI_CONF_LOCAL_SECRET_ENV:-${MINI_CONF_SECRET_ENV:-dev}}}" \
        "${MINI_CONF_LOCAL_TEST_DB_USER:-${MINI_CONF_LOCAL_DB_USER:-${MINI_CONF_DB_USER:-}}}")"
    fi

    test_url="$(build_database_url \
      "test database" \
      "${MINI_CONF_LOCAL_TEST_DATABASE_URL:-}" \
      "${MINI_CONF_LOCAL_TEST_DB_SCHEME:-${MINI_CONF_LOCAL_DB_SCHEME:-${MINI_CONF_DB_SCHEME:-postgres}}}" \
      "${MINI_CONF_LOCAL_TEST_DB_HOST:-${MINI_CONF_LOCAL_DB_HOST:-${MINI_CONF_DB_HOST:-127.0.0.1}}}" \
      "${MINI_CONF_LOCAL_TEST_DB_PORT:-${MINI_CONF_LOCAL_DB_PORT:-${MINI_CONF_DB_PORT:-5432}}}" \
      "${MINI_CONF_LOCAL_TEST_DB_NAME:-}" \
      "${MINI_CONF_LOCAL_TEST_DB_USER:-${MINI_CONF_LOCAL_DB_USER:-${MINI_CONF_DB_USER:-}}}" \
      "${test_password}")"

    if [[ -n "${test_url}" ]]; then
      export TEST_DATABASE_URL="${test_url}"
    fi
  fi
fi

if [[ -z "${INIT_DB_ON_BOOT:-}" ]]; then
  export INIT_DB_ON_BOOT=true
fi
