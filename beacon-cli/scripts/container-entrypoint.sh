#!/usr/bin/env bash
set -euo pipefail

ALLOWED_DOMAINS_FILE="/etc/beacon-code/allowed_domains.txt"
READY_MARKER="/etc/beacon-code/firewall.ready"

if [[ -n "${OPENAI_ALLOWED_DOMAINS:-}" && ! -f "$ALLOWED_DOMAINS_FILE" ]]; then
  domains=()
  for domain in $OPENAI_ALLOWED_DOMAINS; do
    if [[ ! "$domain" =~ ^[a-zA-Z0-9][a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$ ]]; then
      echo "ERROR: Invalid domain format: $domain" >&2
      exit 1
    fi
    domains+=("$domain")
  done
  if [[ "${#domains[@]}" -eq 0 ]]; then
    echo "ERROR: OPENAI_ALLOWED_DOMAINS is empty." >&2
    exit 1
  fi

  if [[ "$(id -u)" -eq 0 ]]; then
    mkdir -p /etc/beacon-code
    printf '%s\n' "${domains[@]}" > "$ALLOWED_DOMAINS_FILE"
    chmod 444 "$ALLOWED_DOMAINS_FILE"
    chown root:root "$ALLOWED_DOMAINS_FILE"
  else
    if ! command -v sudo >/dev/null 2>&1; then
      echo "ERROR: sudo is required to write $ALLOWED_DOMAINS_FILE." >&2
      exit 1
    fi
    if ! printf '%s\n' "${domains[@]}" | sudo -n sh -c 'mkdir -p /etc/beacon-code && cat > /etc/beacon-code/allowed_domains.txt && chmod 444 /etc/beacon-code/allowed_domains.txt && chown root:root /etc/beacon-code/allowed_domains.txt'; then
      echo "ERROR: failed to write allowed domains file via sudo." >&2
      exit 1
    fi
  fi
fi

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
