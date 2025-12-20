# code-core

This crate implements the business logic for Beacon. It is designed to be used by the various Beacon UIs written in Rust.

See also: [Slash Commands](../../docs/slash-commands.md) for a complete list of
interactive commands available in the TUI.

## Dependencies

Note that `code-core` makes some assumptions about certain helper utilities being available in the environment. Currently, this

### macOS

Expects `/usr/bin/sandbox-exec` to be present.

### Linux

Expects the binary containing `code-core` to run the equivalent of `code debug landlock` when `arg0` is `code-linux-sandbox`. See the `code-arg0` crate for details.

### All Platforms

Expects the binary containing `code-core` to simulate the virtual `apply_patch` CLI when `arg1` is `--codex-run-as-apply-patch`. See the `code-arg0` crate for details.
