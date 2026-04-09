#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Backward-compatible wrapper for the old script name. New local helper semantics
# live in local-db-env.sh so portable commands no longer imply developer-local
# resolution by default.
# shellcheck source=/dev/null
source "${script_dir}/local-db-env.sh"
