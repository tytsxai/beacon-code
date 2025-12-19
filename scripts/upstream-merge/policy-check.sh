#!/usr/bin/env bash
set -euo pipefail

# Categorize changes for upstream-merge PR review to reduce mistakes.
#
# Output: Markdown report to stdout (and optionally to a file).
#
# Usage:
#   ./scripts/upstream-merge/policy-check.sh
#   ./scripts/upstream-merge/policy-check.sh --range origin/main...HEAD
#   ./scripts/upstream-merge/policy-check.sh --write .github/auto/VERIFY_policy.md
#   ./scripts/upstream-merge/policy-check.sh --fail-on warn
#
# Notes:
# - Rule file is tab-separated; see scripts/upstream-merge/policy-rules.txt.

RANGE="origin/main...HEAD"
WRITE_PATH=""
RULES_FILE="scripts/upstream-merge/policy-rules.txt"
FAIL_ON_SEVERITY=""

usage() {
  cat <<EOF
Usage: $0 [options]

Options:
  --range <git-range>   Git range (default: ${RANGE})
  --write <path>        Write report to path (also prints to stdout)
  --rules <path>        Rules file (default: ${RULES_FILE})
  --fail-on <severity>  Exit non-zero if any matching rule is >= severity (warn|fail)
  -h, --help            Show this help
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

validate_fail_on() {
  case "$1" in
    "") return 0 ;;
    warn|fail) return 0 ;;
    *) die "invalid --fail-on severity: $1 (expected warn or fail)" ;;
  esac
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --range)
        require_arg "$1" "${2:-}"
        RANGE="$2"; shift 2
        ;;
      --write)
        require_arg "$1" "${2:-}"
        WRITE_PATH="$2"; shift 2
        ;;
      --rules)
        require_arg "$1" "${2:-}"
        RULES_FILE="$2"; shift 2
        ;;
      --fail-on)
        require_arg "$1" "${2:-}"
        FAIL_ON_SEVERITY="$2"; shift 2
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

require_repo_root() {
  local root
  root="$(git rev-parse --show-toplevel 2>/dev/null)" || die "not a git repo"
  cd "$root"
}

require_rules_file() {
  local path="$1"
  [[ -f "$path" ]] || die "rules file not found: ${path}"
}

severity_rank() {
  case "$1" in
    info) echo 0 ;;
    warn) echo 1 ;;
    fail) echo 2 ;;
    *) echo 0 ;;
  esac
}

list_changed_files() {
  git diff --name-only "$RANGE" | sed '/^$/d' || true
}

match_rules() {
  local file="$1"
  local matched_category="shared"
  local matched_severity="info"
  local matched_note=""

  while IFS=$'\t' read -r category severity pattern note; do
    [[ -n "${category:-}" ]] || continue
    [[ "${category:0:1}" != "#" ]] || continue
    if [[ "$file" == $pattern ]]; then
      matched_category="$category"
      matched_severity="$severity"
      matched_note="$note"
      break
    fi
  done < "$RULES_FILE"

  echo "${matched_category}"$'\t'"${matched_severity}"$'\t'"${matched_note}"
}

print_section() {
  local title="$1"
  local count="$2"
  local max_lines="$3"
  shift 3

  echo "### ${title}"
  echo
  echo "- Files: ${count}"
  if [[ "$count" -eq 0 ]]; then
    echo
    return 0
  fi

  echo
  local i=0
  while [[ $# -gt 0 ]]; do
    echo "- \`$1\`"
    i=$((i + 1))
    shift
    if [[ "$i" -ge "$max_lines" ]]; then
      if [[ "$count" -gt "$max_lines" ]]; then
        echo "- …"
      fi
      break
    fi
  done
  echo
}

main() {
  parse_args "$@"
  validate_fail_on "$FAIL_ON_SEVERITY"
  require_repo_root
  require_rules_file "$RULES_FILE"

  declare -a upstream_owned=()
  declare -a downstream_owned=()
  declare -a shared=()
  declare -a risk_matches=()

  local fail_threshold=-1
  if [[ -n "${FAIL_ON_SEVERITY:-}" ]]; then
    fail_threshold="$(severity_rank "$FAIL_ON_SEVERITY")"
  fi
  local fail_hits=0

  while IFS= read -r file; do
    [[ -n "$file" ]] || continue
    match="$(match_rules "$file")"
    category="$(printf "%s" "$match" | cut -f1)"
    severity="$(printf "%s" "$match" | cut -f2)"
    note="$(printf "%s" "$match" | cut -f3-)"

    if [[ "$(severity_rank "$severity")" -ge 1 ]]; then
      if [[ -n "${note:-}" ]]; then
        risk_matches+=("${severity}\t${file}\t${note}")
      else
        risk_matches+=("${severity}\t${file}")
      fi
    fi

    if [[ "$fail_threshold" -ge 0 ]] && [[ "$(severity_rank "$severity")" -ge "$fail_threshold" ]]; then
      fail_hits=$((fail_hits + 1))
    fi

    case "$category" in
      upstream_owned) upstream_owned+=("$file") ;;
      downstream_owned) downstream_owned+=("$file") ;;
      shared) shared+=("$file") ;;
      *) shared+=("$file") ;;
    esac
  done < <(list_changed_files)

  local total_count
  total_count="$((${#upstream_owned[@]} + ${#downstream_owned[@]} + ${#shared[@]}))"

  {
    echo "## Upstream merge policy report"
    echo
    echo "- Range: \`${RANGE}\`"
    echo "- Rules: \`${RULES_FILE}\`"
    echo "- Total changed files: ${total_count}"
    echo
    echo "### Review guidance"
    echo
    echo "- **Upstream-owned**: Prefer staying close to upstream; avoid product-specific behavior unless intentionally required."
    echo "- **Downstream-owned**: Keep fork-specific behavior/docs; avoid being overwritten by upstream."
    echo "- **Shared**: Must be manually reviewed; preserve downstream behavior while adopting upstream fixes."
    echo

    print_section "Upstream-owned" "${#upstream_owned[@]}" 30 "${upstream_owned[@]+"${upstream_owned[@]}"}"
    print_section "Downstream-owned" "${#downstream_owned[@]}" 30 "${downstream_owned[@]+"${downstream_owned[@]}"}"
    print_section "Shared" "${#shared[@]}" 30 "${shared[@]+"${shared[@]}"}"

    echo "### High-risk matches"
    echo
    if [[ "${#risk_matches[@]}" -gt 0 ]]; then
      echo "- Matches (warn/fail): ${#risk_matches[@]}"
      echo
      for row in "${risk_matches[@]}"; do
        severity="$(printf "%s" "$row" | cut -f1)"
        path="$(printf "%s" "$row" | cut -f2)"
        note="$(printf "%s" "$row" | cut -f3-)"
        if [[ -n "${note:-}" ]]; then
          echo "- \`${severity}\` \`${path}\` — ${note}"
        else
          echo "- \`${severity}\` \`${path}\`"
        fi
      done
    else
      echo "- (none)"
    fi
    echo
  } | {
    if [[ -n "${WRITE_PATH:-}" ]]; then
      mkdir -p "$(dirname "$WRITE_PATH")"
      tee "$WRITE_PATH"
    else
      cat
    fi
  }

  if [[ "$fail_threshold" -ge 0 ]] && [[ "$fail_hits" -gt 0 ]]; then
    exit 2
  fi
}

main "$@"
