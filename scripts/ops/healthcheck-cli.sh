#!/usr/bin/env bash
set -euo pipefail

BIN="${1:-beacon}"

if ! command -v "$BIN" >/dev/null 2>&1; then
  echo "CLI binary not found: $BIN" >&2
  exit 1
fi

"$BIN" --version >/dev/null
"$BIN" doctor >/dev/null

echo "ok"
