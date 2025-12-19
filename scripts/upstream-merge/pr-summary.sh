#!/usr/bin/env bash
set -euo pipefail

# Generate a ready-to-paste Markdown summary for an upstream merge PR.
#
# Usage:
#   ./scripts/upstream-merge/pr-summary.sh
#   ./scripts/upstream-merge/pr-summary.sh --base origin/main --upstream upstream/main

BASE_REF="origin/main"
UPSTREAM_REF="upstream/main"

usage() {
  cat <<EOF
Usage: $0 [options]

Options:
  --base <ref>       Base ref for PR comparison (default: ${BASE_REF})
  --upstream <ref>   Upstream ref merged in (default: ${UPSTREAM_REF})
  -h, --help         Show this help
EOF
}

die() {
  echo "error: $*" >&2
  exit 1
}

require_arg() {
  local flag="$1"
  local value="${2:-}"
  if [[ -z "$value" ]]; then
    die "missing value for ${flag}"
  fi
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --base)
        require_arg "$1" "${2:-}"
        BASE_REF="$2"; shift 2
        ;;
      --upstream)
        require_arg "$1" "${2:-}"
        UPSTREAM_REF="$2"; shift 2
        ;;
      -h|--help)
        usage; exit 0
        ;;
      *)
        die "unknown arg: $1"
        ;;
    esac
  done
}

require_ref() {
  local ref="$1"
  git rev-parse --verify "${ref}^{commit}" >/dev/null 2>&1 || die "ref '${ref}' not found; did you fetch it?"
}

short() {
  git rev-parse --short "$1"
}

main() {
  parse_args "$@"

  local root
  root="$(git rev-parse --show-toplevel 2>/dev/null)" || die "not a git repo"
  cd "$root"

  require_ref HEAD
  require_ref "$BASE_REF"
  require_ref "$UPSTREAM_REF"

  local branch
  branch="$(git branch --show-current || true)"

  local head base upstream merge_base files_changed stats
  head="$(short HEAD)"
  base="$(short "$BASE_REF")"
  upstream="$(short "$UPSTREAM_REF")"
  merge_base="$(git merge-base "$BASE_REF" HEAD 2>/dev/null || true)"
  files_changed="$(git diff --name-only "$BASE_REF...HEAD" | wc -l | tr -d ' ')"
  stats="$(git diff --shortstat "$BASE_REF...HEAD" || true)"

  local verify_json=".github/auto/VERIFY.json"
  local verify_status="(not run)"
  if [[ -f "$verify_json" ]]; then
    verify_status="see ${verify_json}"
  fi

  local policy_md=".github/auto/VERIFY_policy.md"
  local policy_status="(not generated)"
  if [[ -f "$policy_md" ]]; then
    policy_status="see ${policy_md}"
  fi

  local latest_log=""
  latest_log="$(ls -t docs/maintenance/upstream-merge-logs/merge-*.md 2>/dev/null | head -1 || true)"
  local latest_log_line="(not created)"
  if [[ -n "${latest_log:-}" ]]; then
    latest_log_line="${latest_log}"
  fi

  local merge_base_short="-"
  if [[ -n "${merge_base:-}" ]]; then
    merge_base_short="$(short "$merge_base")"
  fi

  cat <<EOF
## Upstream merge summary

- Base: \`${BASE_REF}\` (\`${base}\`)
- Branch: \`${branch:-"(detached)"}\` (\`${head}\`)
- Upstream ref: \`${UPSTREAM_REF}\` (\`${upstream}\`)
- Merge-base (base..HEAD): \`${merge_base_short}\`

### Diff stats

- \`${files_changed}\` files changed
- ${stats}

### Verification

- \`./scripts/upstream-merge/verify.sh\`: ${verify_status}
- Policy report: ${policy_status}

### Merge log

- ${latest_log_line}

### Notable changes (top-level)
EOF

  echo
  git diff --dirstat=files,0 "$BASE_REF...HEAD" | sed -e 's/^/- /' | sed -n '1,20p'
}

main "$@"
