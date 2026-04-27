#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

package_name="${MINI_CONF_RELEASE_NAME:-mini-conf-linux-x86_64}"
dist_root="${MINI_CONF_DIST_DIR:-dist}"
staging_dir="${dist_root}/mini-conf"
archive_path="${dist_root}/${package_name}.tar.gz"
target_dir="${CARGO_TARGET_DIR:-target}"
server_binary="${target_dir}/release/server"

if [ ! -f apps/web/package.json ]; then
  echo "apps/web/package.json is required for release packaging" >&2
  exit 1
fi

if [ ! -d migrations ]; then
  echo "migrations directory is required for release packaging" >&2
  exit 1
fi

if [ ! -f deploy/mini-conf.env.example ] || [ ! -f deploy/mini-conf.service.example ]; then
  echo "deploy examples are required for release packaging" >&2
  exit 1
fi

pnpm --dir apps/web build
cargo build --release -p server --bin server

if [ ! -x "${server_binary}" ]; then
  echo "release binary not found at ${server_binary}" >&2
  exit 1
fi

rm -rf "${staging_dir}" "${archive_path}"
mkdir -p \
  "${staging_dir}/bin" \
  "${staging_dir}/web" \
  "${staging_dir}/migrations" \
  "${staging_dir}/config" \
  "${staging_dir}/systemd"

cp "${server_binary}" "${staging_dir}/bin/mini-conf-server"
cp -R apps/web/dist/. "${staging_dir}/web/"
cp -R migrations/. "${staging_dir}/migrations/"
cp deploy/mini-conf.env.example "${staging_dir}/config/mini-conf.env.example"
cp deploy/mini-conf.service.example "${staging_dir}/systemd/mini-conf.service.example"

{
  printf 'name=mini-conf\n'
  printf 'package=%s\n' "${package_name}.tar.gz"
  printf 'git_commit=%s\n' "$(git rev-parse --short=12 HEAD 2>/dev/null || printf 'unknown')"
  printf 'built_at_utc=%s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
} >"${staging_dir}/RELEASE.txt"

tar -C "${dist_root}" -czf "${archive_path}" mini-conf

echo "release package written to ${archive_path}"
