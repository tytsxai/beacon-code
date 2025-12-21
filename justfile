set working-directory := "code-rs"
set positional-arguments

# Display help
help:
    just -l

# `code`
alias c := code
code *args:
    cargo run --bin code -- "$@"

# `code exec`
exec *args:
    cargo run --bin code -- exec "$@"

# `code tui`
tui *args:
    cargo run --bin code -- tui "$@"

# Run the CLI version of the file-search crate.
file-search *args:
    cargo run --bin code-file-search -- "$@"

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
    cargo run -p code-mcp-server -- "$@"

# Bootstrap local dev dependencies (runs from repo root).
bootstrap *args:
    python3 ../scripts/bootstrap.py "$@"

# Repo-wide formatter (Rust + Prettier)
fmt-all:
    cargo fmt -- --config imports_granularity=Item
    bash -lc 'cd .. && pnpm -w run format:fix'

# Stage npm tarballs (requires a rust-release workflow run for the version unless --workflow-url is passed).
stage-npm version *packages:
    python3 ../scripts/stage_npm_packages.py --release-version "{{version}}" {{packages}}
