#!/usr/bin/env bash
set -euo pipefail

code_home="${CODE_HOME:-$HOME/.code}"
fix="false"

usage() {
  cat <<'EOF'
Usage:
  scripts/ops/verify-code-home.sh [--code-home <dir>] [--fix]

Notes:
  - Checks CODE_HOME permissions for sensitive files and directories.
  - Use --fix to apply restrictive permissions (700 for dirs, 600 for files).
EOF
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --code-home)
      if [[ -z "${2:-}" ]]; then
        echo "Missing value for --code-home" >&2
        usage
        exit 1
      fi
      code_home="$2"
      shift 2
      ;;
    --fix)
      fix="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ ! -d "$code_home" ]]; then
  echo "CODE_HOME not found: $code_home" >&2
  exit 1
fi

stat_mode() {
  local path="$1"
  if stat -f "%OLp" "$path" >/dev/null 2>&1; then
    stat -f "%OLp" "$path"
    return
  fi
  if stat -c "%a" "$path" >/dev/null 2>&1; then
    stat -c "%a" "$path"
    return
  fi
  return 1
}

mode_insecure() {
  local mode_str="$1"
  if [[ -z "$mode_str" ]]; then
    return 1
  fi
  local mode=$((10#$mode_str))
  local world=$((mode % 10))
  local group=$(((mode / 10) % 10))
  if (( group != 0 || world != 0 )); then
    return 0
  fi
  return 1
}

issues=0

check_path() {
  local label="$1"
  local path="$2"
  local chmod_mode="$3"

  if [[ ! -e "$path" ]]; then
    return 0
  fi

  local mode=""
  if ! mode="$(stat_mode "$path")"; then
    echo "WARN: Unable to read permissions for $label ($path)" >&2
    issues=$((issues + 1))
    return 0
  fi

  if mode_insecure "$mode"; then
    echo "WARN: $label has insecure permissions ($mode) at $path" >&2
    issues=$((issues + 1))
    if [[ "$fix" == "true" ]]; then
      chmod "$chmod_mode" "$path"
      echo "FIXED: $label set to $chmod_mode at $path" >&2
    fi
  fi
}

check_path "CODE_HOME" "$code_home" "700"

sensitive_files=(
  "auth.json"
  "auth_accounts.json"
  "config.toml"
  "managed_config.toml"
  "history.jsonl"
  ".env"
)

sensitive_dirs=(
  "sessions"
  "log"
  "logs"
  "rules"
  "prompts"
  "checkpoints"
  "working"
)

for item in "${sensitive_files[@]}"; do
  check_path "$item" "$code_home/$item" "600"
done

for item in "${sensitive_dirs[@]}"; do
  check_path "$item" "$code_home/$item" "700"
done

if (( issues > 0 )); then
  echo "code home permission check: FAILED (${issues} issue(s))" >&2
  exit 2
fi

echo "ok"
