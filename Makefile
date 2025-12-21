# Beacon Code - developer convenience targets.

.PHONY: help bootstrap build build-dev build-fast fmt fmt-rust fmt-js clippy test audit doctor install install-dev install-modules release-local

help:
	@echo "Beacon Code - common dev targets"
	@echo ""
	@echo "Setup:"
	@echo "  make bootstrap       Install/check local prerequisites"
	@echo "  make install-dev     Install dev tools (just, cargo-nextest, etc.)"
	@echo ""
	@echo "Build:"
	@echo "  make build           Release build (Rust bins)"
	@echo "  make build-dev       Debug build (Rust bins)"
	@echo "  make build-fast      Fast local build (profile dev-fast)"
	@echo ""
	@echo "Quality:"
	@echo "  make fmt             Format Rust + repo files"
	@echo "  make clippy          Run clippy (all-features)"
	@echo "  make test            Run Rust tests (all-features)"
	@echo "  make audit           Run cargo-audit"
	@echo ""
	@echo "Install:"
	@echo "  make install         Install CLI to ~/.beacon-code"
	@echo "  make install-modules List available install modules"
	@echo "  make release-local   Build local release package"
	@echo ""
	@echo "Notes:"
	@echo "  - Most Rust helpers are also available via: just -l"

bootstrap:
	@./scripts/bootstrap.sh

build:
	@cd code-rs && cargo build --locked --profile release-prod --bin code --bin code-tui --bin code-exec

build-dev:
	@cd code-rs && cargo build --locked --bin code --bin code-tui --bin code-exec

build-fast:
	@./build-fast.sh

fmt: fmt-rust fmt-js

fmt-rust:
	@cd code-rs && cargo fmt -- --config imports_granularity=Item

fmt-js:
	@command -v pnpm >/dev/null 2>&1 && pnpm -w run format:fix || echo "pnpm not found; skipping JS formatting"

clippy:
	@cd code-rs && cargo clippy --all-features --tests -- -D warnings

test:
	@cd code-rs && cargo test --all-features

audit:
	@cd code-rs && cargo audit

doctor:
	@cd code-rs && cargo run -p code-cli --bin code -- doctor || true

# Install targets
install:
	@python3 scripts/install.py --module cli --verbose

install-dev:
	@python3 scripts/install.py --module dev-tools --verbose

install-modules:
	@python3 scripts/install.py --list-modules

release-local:
	@echo "Building local release package..."
	@$(MAKE) build
	@mkdir -p dist/local
	@cp code-rs/target/release-prod/code dist/local/ 2>/dev/null || cp code-rs/target/release/code dist/local/
	@echo "Release binary: dist/local/code"
