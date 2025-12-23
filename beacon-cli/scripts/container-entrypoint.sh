#!/usr/bin/env bash
set -euo pipefail

READY_MARKER="/etc/beacon-code/firewall.ready"

if [[ ! -f "$READY_MARKER" ]]; then
  if [[ ! -x /usr/local/bin/init_firewall.sh ]]; then
    echo "ERROR: /usr/local/bin/init_firewall.sh is missing or not executable." >&2
    exit 1
  fi

  if [[ "$(id -u)" -eq 0 ]]; then
    /usr/local/bin/init_firewall.sh
  else
    if ! command -v sudo >/dev/null 2>&1; then
      echo "ERROR: sudo is required to initialize the firewall as root." >&2
      exit 1
    fi
    if ! sudo -n /usr/local/bin/init_firewall.sh; then
      echo "ERROR: failed to initialize firewall. Ensure NET_ADMIN/NET_RAW caps and sudoers allow init_firewall.sh." >&2
      exit 1
    fi
  fi
fi

if [[ "$#" -eq 0 ]]; then
  set -- beacon
fi

exec "$@"
