#!/usr/bin/env bash
set -euo pipefail

backup_in=""
require_worktrees="false"
verbose="false"

usage() {
  cat <<'EOF'
Usage:
  scripts/ops/verify-backup.sh --in <backup.tgz> [--require-worktrees] [--verbose]

Notes:
  - Verifies the backup archive is readable and contains expected entries.
  - Use --require-worktrees to fail if working/ is missing.
EOF
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --in)
      if [[ -z "${2:-}" ]]; then
        echo "Missing value for --in" >&2
        usage
        exit 1
      fi
      backup_in="$2"
      shift 2
      ;;
    --require-worktrees)
      require_worktrees="true"
      shift
      ;;
    --verbose)
      verbose="true"
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

if [[ -z "$backup_in" ]]; then
  echo "--in is required" >&2
  usage
  exit 1
fi

if [[ ! -f "$backup_in" ]]; then
  echo "Backup archive not found: $backup_in" >&2
  exit 1
fi

if ! tar -tzf "$backup_in" >/dev/null 2>&1; then
  echo "Backup archive is not readable: $backup_in" >&2
  exit 1
fi

contents="$(tar -tzf "$backup_in")"
if [[ -z "$contents" ]]; then
  echo "Backup archive is empty: $backup_in" >&2
  exit 1
fi

expected=(
  "auth.json"
  "auth_accounts.json"
  "history.jsonl"
  "sessions/"
  "log/"
  "logs/"
  "config.toml"
  "managed_config.toml"
  ".env"
  "rules/"
  "prompts/"
  "checkpoints/"
  "cache.json"
  "working/"
)

found=()
for item in "${expected[@]}"; do
  if [[ "$item" == */ ]]; then
    if printf '%s\n' "$contents" | grep -q "^${item}"; then
      found+=("$item")
    fi
  else
    if printf '%s\n' "$contents" | grep -Fxq "$item"; then
      found+=("$item")
    fi
  fi
done

if (( ${#found[@]} == 0 )); then
  echo "Backup archive does not include any expected entries." >&2
  exit 2
fi

if [[ "$require_worktrees" == "true" ]] && ! printf '%s\n' "${found[@]}" | grep -Fxq "working/"; then
  echo "Backup archive missing working/ (required by --require-worktrees)." >&2
  exit 2
fi

if [[ "$verbose" == "true" ]]; then
  echo "Found entries:"
  printf ' - %s\n' "${found[@]}"
fi

echo "ok"
