#!/usr/bin/env bash
set -euo pipefail

SOCKET_PATH="${1:-/run/code-app-server/app-server.sock}"

if ! command -v socat >/dev/null 2>&1; then
  echo "socat is required for the app-server healthcheck" >&2
  exit 1
fi

payload=$(cat <<'JSON'
{"id":0,"method":"initialize","params":{"clientInfo":{"name":"healthcheck","title":"Healthcheck","version":"0.0.0"}}}
{"method":"initialized","params":{}}
JSON
)

response=$(printf '%s\n' "$payload" | socat - "UNIX-CONNECT:${SOCKET_PATH}" | head -n 1 || true)

if [ -z "$response" ]; then
  echo "No response from app-server at ${SOCKET_PATH}" >&2
  exit 1
fi

if ! printf '%s' "$response" | grep -q '"result"'; then
  echo "Unexpected response: $response" >&2
  exit 1
fi

echo "ok"
