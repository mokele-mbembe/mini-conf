#!/usr/bin/env bash
set -euo pipefail

archive_path="${1:-${MINI_CONF_RELEASE_ARCHIVE:-dist/mini-conf-linux-x86_64.tar.gz}}"

if [ ! -f "${archive_path}" ]; then
  echo "release archive not found: ${archive_path}" >&2
  echo "Run: just release-package" >&2
  exit 1
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/mini-conf-release-check.XXXXXX")"
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

entries_file="${tmp_dir}/entries.txt"
tar -tzf "${archive_path}" >"${entries_file}"

if grep -Eq '(^/|(^|/)\.\.(/|$))' "${entries_file}"; then
  echo "release archive contains unsafe paths" >&2
  exit 1
fi

if grep -Ev '^mini-conf(/|$)' "${entries_file}" >/dev/null; then
  echo "release archive must contain only the mini-conf/ root directory" >&2
  exit 1
fi

require_entry() {
  local entry="$1"
  if ! grep -Fxq "${entry}" "${entries_file}"; then
    echo "release archive is missing ${entry}" >&2
    exit 1
  fi
}

require_entry "mini-conf/bin/mini-conf-server"
require_entry "mini-conf/web/index.html"
require_entry "mini-conf/config/mini-conf.env.example"
require_entry "mini-conf/systemd/mini-conf.service.example"
require_entry "mini-conf/RELEASE.txt"

if ! grep -Eq '^mini-conf/migrations/[0-9]+_.+\.up\.sql$' "${entries_file}"; then
  echo "release archive must include migration .up.sql files" >&2
  exit 1
fi

if ! grep -Eq '^mini-conf/migrations/[0-9]+_.+\.down\.sql$' "${entries_file}"; then
  echo "release archive must include migration .down.sql files" >&2
  exit 1
fi

tar -xzf "${archive_path}" -C "${tmp_dir}"

package_root="${tmp_dir}/mini-conf"
binary_path="${package_root}/bin/mini-conf-server"
env_example="${package_root}/config/mini-conf.env.example"
service_example="${package_root}/systemd/mini-conf.service.example"
release_file="${package_root}/RELEASE.txt"

if [ ! -x "${binary_path}" ]; then
  echo "mini-conf-server must be executable" >&2
  exit 1
fi

if [ ! -s "${package_root}/web/index.html" ]; then
  echo "web/index.html must be non-empty" >&2
  exit 1
fi

grep -Fxq "APP_ENV=prod" "${env_example}" || {
  echo "env example must set APP_ENV=prod" >&2
  exit 1
}

grep -Fxq "INIT_DB_ON_BOOT=false" "${env_example}" || {
  echo "env example must keep INIT_DB_ON_BOOT=false" >&2
  exit 1
}

grep -Eq '^DATABASE_URL=postgres://' "${env_example}" || {
  echo "env example must include an external PostgreSQL DATABASE_URL" >&2
  exit 1
}

grep -Fxq "ExecStart=/opt/mini-conf/bin/mini-conf-server" "${service_example}" || {
  echo "systemd example must start /opt/mini-conf/bin/mini-conf-server" >&2
  exit 1
}

grep -Fxq "NoNewPrivileges=true" "${service_example}" || {
  echo "systemd example must keep NoNewPrivileges=true" >&2
  exit 1
}

grep -Eq '^git_commit=' "${release_file}" || {
  echo "RELEASE.txt must include git_commit" >&2
  exit 1
}

echo "release package check passed: ${archive_path}"
