#!/usr/bin/env bash
set -euo pipefail

# Safe helper for the repo's upstream merge-train workflow.
#
# Defaults:
# - Requires a clean working tree
# - Creates a dedicated sync branch (never operates directly on main)
# - Previews merge conflicts (without committing) unless --no-preview is set
#
# Usage:
#   ./scripts/upstream-sync.sh
#   ./scripts/upstream-sync.sh --log
#   ./scripts/upstream-sync.sh --merge --verify --log

BASE_BRANCH="main"
UPSTREAM_REF="upstream/main"
SYNC_BRANCH="sync/upstream-$(date +%Y%m%d)"
DO_MERGE=false
DO_PREVIEW=true
DO_VERIFY=false
DO_LOG=false

usage() {
  cat <<EOF
Usage: $0 [options]

Options:
  --base <branch>     Base branch (default: ${BASE_BRANCH})
  --upstream <ref>    Upstream ref to merge (default: ${UPSTREAM_REF})
  --branch <name>     Sync branch name (default: ${SYNC_BRANCH})
  --merge             Perform merge (default: preview only)
  --no-preview        Skip preview conflict listing
  --verify            Run ./scripts/upstream-merge/verify.sh after merge
  --log               Initialize merge log via scripts/upstream-merge/log-merge.sh
  -h, --help          Show this help
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
        BASE_BRANCH="$2"; shift 2
        ;;
      --upstream)
        require_arg "$1" "${2:-}"
        UPSTREAM_REF="$2"; shift 2
        ;;
      --branch)
        require_arg "$1" "${2:-}"
        SYNC_BRANCH="$2"; shift 2
        ;;
      --merge)
        DO_MERGE=true; shift
        ;;
      --no-preview)
        DO_PREVIEW=false; shift
        ;;
      --verify)
        DO_VERIFY=true; shift
        ;;
      --log)
        DO_LOG=true; shift
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

ensure_repo_root() {
  local root
  root="$(git rev-parse --show-toplevel 2>/dev/null)" || die "not a git repo"
  cd "$root"
}

require_clean_worktree() {
  if [[ -n "$(git status --porcelain)" ]]; then
    die "working tree not clean; commit/stash changes before syncing upstream"
  fi
}

require_remote() {
  local name="$1"
  git remote get-url "$name" >/dev/null 2>&1 || die "missing remote '${name}' (run: git remote add ${name} <url>)"
}

require_ref() {
  local ref="$1"
  git rev-parse --verify "${ref}^{commit}" >/dev/null 2>&1 || die "ref '${ref}' not found; did you fetch it?"
}

ensure_rerere() {
  local enabled
  enabled="$(git config --bool rerere.enabled || echo false)"
  if [[ "$enabled" != true ]]; then
    git config rerere.enabled true
    echo "[sync] enabled git rerere (conflict resolution reuse)"
  fi
}

main() {
  parse_args "$@"
  ensure_repo_root

  require_clean_worktree
  require_remote origin
  require_remote upstream
  ensure_rerere

  echo "[sync] fetching remotes..."
  git fetch origin --prune
  git fetch upstream --prune
  require_ref "$UPSTREAM_REF"

  echo "[sync] updating base branch '${BASE_BRANCH}'..."
  git checkout "$BASE_BRANCH" >/dev/null 2>&1 || die "base branch '${BASE_BRANCH}' not found"
  git pull --ff-only origin "$BASE_BRANCH"

  if git show-ref --verify --quiet "refs/heads/${SYNC_BRANCH}"; then
    die "branch '${SYNC_BRANCH}' already exists; choose a different --branch"
  fi

  if [[ "$SYNC_BRANCH" != upstream-merge && "$SYNC_BRANCH" != sync/upstream-* ]]; then
    echo "[sync] warning: branch '${SYNC_BRANCH}' does not match 'sync/upstream-YYYYMMDD' or 'upstream-merge'"
  fi

  echo "[sync] creating sync branch: ${SYNC_BRANCH}"
  git checkout -b "$SYNC_BRANCH"

  echo "[sync] upstream commit: $(git rev-parse --short "$UPSTREAM_REF")"
  echo "[sync] base commit:    $(git rev-parse --short "$BASE_BRANCH")"

  if [[ "$DO_LOG" == true ]]; then
    echo "[sync] initializing merge log..."
    log_file="$(./scripts/upstream-merge/log-merge.sh init "$UPSTREAM_REF")" || exit 1
    echo "[sync] merge log: ${log_file}"
  fi

  if [[ "$DO_PREVIEW" == true ]]; then
    echo "[sync] previewing merge to list conflict files..."
    set +e
    git merge --no-commit --no-ff "$UPSTREAM_REF"
    rc=$?
    set -e
    if [[ $rc -ne 0 ]]; then
      echo "[sync] merge preview reported conflicts (expected sometimes)."
      git diff --name-only --diff-filter=U | sed -e 's/^/[conflict] /'
      echo "[sync] aborting preview merge (no changes committed)."
      git merge --abort
    else
      echo "[sync] preview merge clean; aborting preview merge (no changes committed)."
      git merge --abort
    fi
  fi

  if [[ "$DO_MERGE" != true ]]; then
    cat <<EOF

[sync] Prepared branch '${SYNC_BRANCH}'.

Next steps (recommended):
  1) Perform the merge:   git merge --no-ff ${UPSTREAM_REF}
  2) Resolve conflicts (if any)
  3) Verify:             ./scripts/upstream-merge/verify.sh
  4) Policy report:      ./scripts/upstream-merge/policy-check.sh --range origin/${BASE_BRANCH}...HEAD
  5) PR summary:         ./scripts/upstream-merge/pr-summary.sh
  6) Log decisions:      ./scripts/upstream-merge/log-merge.sh note/decision/...
  7) Push + PR:          git push -u origin ${SYNC_BRANCH}

See docs/upstream-sync.md for the full SOP.
EOF
    exit 0
  fi

  echo "[sync] merging '${UPSTREAM_REF}' into '${SYNC_BRANCH}'..."
  git merge --no-ff "$UPSTREAM_REF"

  if [[ "$DO_VERIFY" == true ]]; then
    echo "[sync] running verification: ./scripts/upstream-merge/verify.sh"
    ./scripts/upstream-merge/verify.sh
  fi

  cat <<EOF

[sync] Merge complete on '${SYNC_BRANCH}'.

Next steps:
  1) Policy:   ./scripts/upstream-merge/policy-check.sh --range origin/${BASE_BRANCH}...HEAD
  2) Summary:  ./scripts/upstream-merge/pr-summary.sh
  3) Push:     git push -u origin ${SYNC_BRANCH}
  4) Open PR:  merge '${SYNC_BRANCH}' into '${BASE_BRANCH}' (keep merge commit)

EOF
}

main "$@"
