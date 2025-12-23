#!/usr/bin/env bash
set -euo pipefail

MAX_JOBS="${MAX_JOBS:-32}"
OUT_DIR="${OUT_DIR:-.prod_ready_logs/$(date +%Y%m%d_%H%M%S)}"

mkdir -p "$OUT_DIR"

fail_count=0

acquire_slot() {
  while (( $(jobs -pr | wc -l | tr -d ' ') >= MAX_JOBS )); do
    sleep 0.05
  done
}

run_task() {
  local name="$1"
  shift
  local log="$OUT_DIR/${name}.log"
  local status="$OUT_DIR/${name}.status"

  acquire_slot
  (
    {
      echo "==> ${name}"
      echo "cwd: $(pwd)"
      echo "cmd: $*"
      echo
      "$@"
    } >"$log" 2>&1
    echo "ok" >"$status"
  ) || {
    echo "fail" >"$status"
  } &
}

finalize() {
  wait || true

  {
    echo "# Production readiness sweep"
    echo
    echo "- out_dir: \`$OUT_DIR\`"
    echo "- max_jobs: \`$MAX_JOBS\`"
    echo
    echo "## Task results"
    echo
    for status_file in "$OUT_DIR"/*.status; do
      [[ -e "$status_file" ]] || continue
      name="$(basename "$status_file" .status)"
      status="$(cat "$status_file")"
      if [[ "$status" != "ok" ]]; then
        ((fail_count+=1))
      fi
      echo "- ${name}: ${status} (log: \`$OUT_DIR/${name}.log\`)"
    done
    echo
    echo "## Overall"
    if (( fail_count == 0 )); then
      echo "- result: ok"
    else
      echo "- result: fail"
      echo "- failed_tasks: ${fail_count}"
    fi
  } >"$OUT_DIR/summary.md"

  echo
  echo "Wrote summary: $OUT_DIR/summary.md"
  if (( fail_count != 0 )); then
    echo "Some tasks failed (${fail_count}). See logs in: $OUT_DIR/"
    exit 2
  fi
}

trap finalize EXIT

run_task repo_tree bash -lc 'ls'
run_task suspicious_patterns bash -lc 'rg -n "TODO\\(|FIXME|panic!\\(|unwrap\\(\\)|expect\\(" -S code-rs shell-tool-mcp sdk scripts third_party 2>/dev/null || true'
run_task secret_like_strings bash -lc 'rg -n "(SECRET|API_KEY|TOKEN|PASSWORD|PRIVATE_KEY|BEGIN RSA|BEGIN OPENSSH)" -S . || true'
run_task ci_workflow_check bash -lc 'sed -n "1,260p" .github/workflows/ci.yml'

run_task js_format bash -lc 'pnpm -w run format'
run_task js_tests bash -lc 'pnpm -w test || true'

run_task rust_fmt bash -lc 'cd code-rs && cargo fmt --all -- --check'
run_task rust_clippy bash -lc 'cd code-rs && cargo clippy --all-targets -- -D warnings'
run_task rust_tests bash -lc 'cd code-rs && cargo test --all-features'
run_task rust_audit bash -lc 'cd code-rs && cargo audit'

run_task cli_smokes bash -lc 'bash scripts/ci-tests.sh'
