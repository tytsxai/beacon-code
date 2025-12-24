#!/usr/bin/env bash
set -euo pipefail

SOCKET_PATH="/run/code-app-server/app-server.sock"
timeout_seconds="${HEALTHCHECK_TIMEOUT_SECONDS:-5}"

usage() {
  cat <<'EOF'
Usage:
  scripts/ops/healthcheck-app-server.sh [socket_path] [--timeout <seconds>]

Notes:
  - socket_path defaults to /run/code-app-server/app-server.sock
  - Set HEALTHCHECK_TIMEOUT_SECONDS to override the default timeout (5s).
EOF
}

socket_set="false"
while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --timeout)
      if [[ -z "${2:-}" ]]; then
        echo "Missing value for --timeout" >&2
        usage
        exit 1
      fi
      timeout_seconds="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      if [[ "$socket_set" == "false" ]]; then
        SOCKET_PATH="$1"
        socket_set="true"
        shift
      else
        echo "Unknown argument: $1" >&2
        usage
        exit 1
      fi
      ;;
  esac
done

if ! command -v socat >/dev/null 2>&1; then
  echo "socat is required for the app-server healthcheck" >&2
  exit 1
fi

payload=$(cat <<'JSON'
{"id":0,"method":"initialize","params":{"clientInfo":{"name":"healthcheck","title":"Healthcheck","version":"0.0.0"}}}
{"method":"initialized","params":{}}
JSON
)

response=$(printf '%s\n' "$payload" | socat -T "$timeout_seconds" - "UNIX-CONNECT:${SOCKET_PATH}" | head -n 1 || true)

if [ -z "$response" ]; then
  echo "No response from app-server at ${SOCKET_PATH}" >&2
  exit 1
fi

if ! printf '%s' "$response" | grep -q '"result"'; then
  echo "Unexpected response: $response" >&2
  exit 1
fi

echo "ok"
