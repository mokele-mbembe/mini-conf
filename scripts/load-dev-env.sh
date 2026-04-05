#!/usr/bin/env bash

# Load optional local development environment customizations for hooks and
# local commands without hard-coding a machine-specific path into repo config.

if [[ -n "${MINI_CONF_DEV_ENV_FILE:-}" ]]; then
  if [[ -f "${MINI_CONF_DEV_ENV_FILE}" ]]; then
    # shellcheck source=/dev/null
    source "${MINI_CONF_DEV_ENV_FILE}"
  fi
elif [[ -f "${XDG_CONFIG_HOME:-$HOME/.config}/mini-conf/dev-env.sh" ]]; then
  # shellcheck source=/dev/null
  source "${XDG_CONFIG_HOME:-$HOME/.config}/mini-conf/dev-env.sh"
elif [[ -f "${XDG_CONFIG_HOME:-$HOME/.config}/mini-conf/activate-fedora43.sh" ]]; then
  # Backward compatibility with the earlier local filename.
  # shellcheck source=/dev/null
  source "${XDG_CONFIG_HOME:-$HOME/.config}/mini-conf/activate-fedora43.sh"
fi
