#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

usage() {
  cat <<'EOF'
Usage:
  scripts/code-home-backup.sh backup --out <path> [--code-home <dir>] [--include-worktrees]
  scripts/code-home-backup.sh restore --in <path> [--code-home <dir>] [--force]

Notes:
  - Defaults CODE_HOME to $HOME/.code if CODE_HOME is not set.
  - Backup includes: auth.json, auth_accounts.json, history.jsonl, sessions/,
    log/, logs/, config.toml, managed_config.toml, rules/, prompts/, checkpoints/,
    cache.json, and .env (if present).
  - Use --include-worktrees to include working/ worktrees (may be large).
EOF
}

action="${1:-}"
shift || true

code_home="${CODE_HOME:-$HOME/.code}"
backup_out=""
backup_in=""
force_restore="false"
include_worktrees="false"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --code-home)
      code_home="$2"
      shift 2
      ;;
    --out)
      backup_out="$2"
      shift 2
      ;;
    --in)
      backup_in="$2"
      shift 2
      ;;
    --force)
      force_restore="true"
      shift
      ;;
    --include-worktrees)
      include_worktrees="true"
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

if [ -z "$action" ]; then
  usage
  exit 1
fi

case "$action" in
  backup)
    if [ -z "$backup_out" ]; then
      timestamp="$(date +"%Y%m%d-%H%M%S")"
      backup_out="code-home-backup-${timestamp}.tgz"
    fi

    if [ ! -d "$code_home" ]; then
      echo "Error: CODE_HOME not found: $code_home" >&2
      exit 1
    fi

    includes=(
      auth.json
      auth_accounts.json
      history.jsonl
      sessions
      log
      logs
      config.toml
      managed_config.toml
      rules
      prompts
      checkpoints
      cache.json
      .env
    )
    if [ "$include_worktrees" = "true" ]; then
      includes+=(working)
    fi

    existing=()
    for item in "${includes[@]}"; do
      if [ -e "$code_home/$item" ]; then
        existing+=("$item")
      fi
    done

    if [ "${#existing[@]}" -eq 0 ]; then
      echo "Error: No known state found under $code_home" >&2
      exit 1
    fi

    tar -czf "$backup_out" -C "$code_home" "${existing[@]}"
    echo "Backup created: $backup_out"
    ;;
  restore)
    if [ -z "$backup_in" ]; then
      echo "Error: --in is required for restore" >&2
      exit 1
    fi

    if [ ! -f "$backup_in" ]; then
      echo "Error: backup not found: $backup_in" >&2
      exit 1
    fi

    # Reject archives with paths that could escape the target directory.
    # Allow only relative paths without '..' components.
    unsafe_paths="$(tar -tzf "$backup_in" \
      | sed -e 's#^\./##' \
      | awk '
          $0 ~ /^\// { print; next }
          $0 == ".." { print; next }
          $0 ~ /^\.\.$/ { print; next }
          $0 ~ /^\.\.\// { print; next }
          $0 ~ /\/\.\.(\/|$)/ { print; next }
          $0 ~ /\\/ { print; next }
        '
      | head -n 20)"
    if [ -n "$unsafe_paths" ]; then
      echo "Error: backup contains unsafe paths (refusing to extract):" >&2
      echo "$unsafe_paths" >&2
      exit 1
    fi

    mkdir -p "$code_home"

    if [ "$force_restore" != "true" ]; then
      entries="$(tar -tzf "$backup_in" | awk -F/ 'NF{print $1}' | sort -u)"
      while IFS= read -r entry; do
        if [ -e "$code_home/$entry" ]; then
          echo "Error: target exists: $code_home/$entry (use --force to overwrite)" >&2
          exit 1
        fi
      done <<< "$entries"
    fi

    tar -xzf "$backup_in" -C "$code_home" --no-same-owner --no-same-permissions
    echo "Restore completed into: $code_home"
    ;;
  *)
    echo "Unknown action: $action" >&2
    usage
    exit 1
    ;;
esac
