set working-directory := "codex-rs"
set positional-arguments

# Display help
help:
    just -l

# `codex`
alias c := codex
codex *args:
    cargo run --bin codex -- "$@"

# `codex exec`
exec *args:
    cargo run --bin codex -- exec "$@"

# `codex tui`
tui *args:
    cargo run --bin codex -- tui "$@"

# Run the CLI version of the file-search crate.
file-search *args:
    cargo run --bin codex-file-search -- "$@"

# Build the CLI and run the app-server test client
app-server-test-client *args:
    cargo build -p codex-cli
    cargo run -p codex-app-server-test-client -- --codex-bin ./target/debug/codex "$@"

# format code
fmt:
    cargo fmt -- --config imports_granularity=Item

fix *args:
    cargo clippy --fix --all-features --tests --allow-dirty "$@"

clippy:
    cargo clippy --all-features --tests "$@"

install:
    rustup show active-toolchain
    cargo fetch

# Run `cargo nextest` since it's faster than `cargo test`, though including
# --no-fail-fast is important to ensure all tests are run.
#
# Run `cargo install cargo-nextest` if you don't have it installed.
test:
    cargo nextest run --no-fail-fast

# Run the MCP server
mcp-server-run *args:
    cargo run -p codex-mcp-server -- "$@"

# Upstream sync helper (downstream merge-train).
# Canonical SOP: docs/upstream-sync.md
upstream-sync *args:
    ../scripts/upstream-sync.sh "$@"

# Unified verification used by upstream-merge workflow and local sync PRs.
upstream-verify:
    ../scripts/upstream-merge/verify.sh

# Upstream merge log helper (writes to docs/maintenance/upstream-merge-logs/).
upstream-log *args:
    ../scripts/upstream-merge/log-merge.sh "$@"

# Upstream diff helpers (codex-rs vs code-rs).
upstream-diff *args:
    ../scripts/upstream-merge/diff-crates.sh "$@"

upstream-critical *args:
    ../scripts/upstream-merge/highlight-critical-changes.sh "$@"

# Policy report for upstream merge PR review (categorizes changed files).
upstream-policy *args:
    ../scripts/upstream-merge/policy-check.sh "$@"

# Strict policy mode (fails if any warn/fail matches exist).
upstream-policy-strict *args:
    ../scripts/upstream-merge/policy-check.sh --fail-on warn "$@"

# Generate a ready-to-paste PR summary for upstream merges.
upstream-pr-summary *args:
    ../scripts/upstream-merge/pr-summary.sh "$@"
