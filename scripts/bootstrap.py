#!/usr/bin/env python3
"""
Bootstrap a local development environment for Beacon Code.

This script is intentionally conservative:
- It installs Cargo-managed tools (like `just`) when missing.
- It enables Rust components (rustfmt/clippy) when `rustup` is available.
- It only *checks* for non-Cargo tooling (node/pnpm/gh/zstd) and prints guidance.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--install",
        action="store_true",
        help="Install missing Cargo tools (default: on).",
    )
    parser.add_argument(
        "--no-install",
        dest="install",
        action="store_false",
        help="Only check prerequisites; do not install anything.",
    )
    parser.set_defaults(install=True)
    return parser.parse_args()


def run(cmd: list[str], *, cwd: Path | None = None) -> None:
    print("+", " ".join(cmd))
    subprocess.check_call(cmd, cwd=cwd or REPO_ROOT)


def have(cmd: str) -> bool:
    return shutil.which(cmd) is not None


def note(msg: str) -> None:
    print(msg)


def warn(msg: str) -> None:
    print(f"WARNING: {msg}", file=sys.stderr)


def ensure_rust_components() -> None:
    if not have("rustup"):
        warn("rustup not found; skipping rustfmt/clippy install (install Rust via https://rustup.rs/).")
        return
    run(["rustup", "component", "add", "rustfmt"])
    run(["rustup", "component", "add", "clippy"])


def ensure_cargo_tool(tool: str) -> None:
    if have(tool):
        return
    if not have("cargo"):
        raise RuntimeError("cargo not found; install Rust via https://rustup.rs/ first.")
    run(["cargo", "install", "--locked", tool])


def main() -> int:
    args = parse_args()

    note(f"[bootstrap] repo: {REPO_ROOT}")

    if not have("cargo"):
        warn("cargo not found (Rust toolchain missing). Install Rust via https://rustup.rs/.")
    else:
        note(f"[bootstrap] cargo: {subprocess.check_output(['cargo', '--version'], text=True).strip()}")

    ensure_rust_components()

    if args.install:
        for tool in ("just", "cargo-nextest", "cargo-audit"):
            ensure_cargo_tool(tool)
    else:
        note("[bootstrap] --no-install: skipping cargo tool installation")

    # These are optional / environment-specific, but commonly needed.
    if not have("node"):
        warn("node not found (recommended: Node.js 20+).")
    if not have("pnpm"):
        warn("pnpm not found (recommended: `corepack enable` then `corepack prepare pnpm@10.8.1 --activate`).")
    if not have("gh"):
        warn("gh (GitHub CLI) not found; required for some release helpers.")
    if not have("zstd"):
        warn("zstd not found; required to produce/consume .zst release artifacts.")

    # Print a quick next step summary.
    note("")
    note("[bootstrap] next steps:")
    note("  - build (dev):   make build-dev")
    note("  - run (tui):     just tui -- \"explain this codebase to me\"")
    note("  - fast build:    ./build-fast.sh")
    note("  - tests (rust):  (cd code-rs && cargo test --all-features)")
    note("  - format:        just fmt && pnpm -w format:fix")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
