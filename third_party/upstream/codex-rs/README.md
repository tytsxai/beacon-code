# Beacon CLI (Rust Implementation)

We provide Beacon CLI as a standalone, native executable to ensure a zero-dependency install.

## Installing Beacon

Today, the easiest way to install Beacon is via `npm`:

```shell
npm i -g @tytsxai/beacon-code
codex
```

You can also install via Homebrew (`brew install --cask codex`) or download a platform-specific release directly from our [GitHub Releases](https://github.com/tytsxai/beacon-code/releases).

## Documentation quickstart

- First run with Beacon? Follow the walkthrough in [`docs/getting-started.md`](../docs/getting-started.md) for prompts, keyboard shortcuts, and session management.
- Already shipping with Beacon and want deeper control? Jump to [`docs/advanced.md`](../docs/advanced.md) and the configuration reference at [`docs/config.md`](../docs/config.md).

## What's new in the Rust CLI

The Rust implementation is now the maintained Beacon CLI and serves as the default experience. It includes a number of features that the legacy TypeScript CLI never supported.

### Config

Beacon supports a rich set of configuration options. Note that the Rust CLI uses `config.toml` instead of `config.json`. See [`docs/config.md`](../docs/config.md) for details.

### Model Context Protocol Support

#### MCP client

Beacon CLI functions as an MCP client that allows the Beacon CLI and IDE extension to connect to MCP servers on startup. See the [`configuration documentation`](../docs/config.md#mcp_servers) for details.

#### MCP server (experimental)

Beacon can be launched as an MCP _server_ by running `codex mcp-server`. This allows _other_ MCP clients to use Beacon as a tool for another agent.

Use the [`@modelcontextprotocol/inspector`](https://github.com/modelcontextprotocol/inspector) to try it out:

```shell
npx @modelcontextprotocol/inspector codex mcp-server
```

Use `codex mcp` to add/list/get/remove MCP server launchers defined in `config.toml`, and `codex mcp-server` to run the MCP server directly.

### Notifications

You can enable notifications by configuring a script that is run whenever the agent finishes a turn. The [notify documentation](../docs/config.md#notify) includes a detailed example that explains how to get desktop notifications via [terminal-notifier](https://github.com/julienXX/terminal-notifier) on macOS. When Beacon detects that it is running under WSL 2 inside Windows Terminal (`WT_SESSION` is set), the TUI automatically falls back to native Windows toast notifications so approval prompts and completed turns surface even though Windows Terminal does not implement OSC 9.

### `codex exec` to run Beacon programmatically/non-interactively

To run Beacon non-interactively, run `codex exec PROMPT` (you can also pass the prompt via `stdin`) and Beacon will work on your task until it decides that it is done and exits. Output is printed to the terminal directly. You can set the `RUST_LOG` environment variable to see more about what's going on.

### Experimenting with the Beacon Sandbox

To test to see what happens when a command is run under the sandbox provided by Beacon, we provide the following subcommands in Beacon CLI:

```
# macOS
codex sandbox macos [--full-auto] [--log-denials] [COMMAND]...

# Linux
codex sandbox linux [--full-auto] [COMMAND]...

# Windows
codex sandbox windows [--full-auto] [COMMAND]...

# Legacy aliases
codex debug seatbelt [--full-auto] [--log-denials] [COMMAND]...
codex debug landlock [--full-auto] [COMMAND]...
```

### Selecting a sandbox policy via `--sandbox`

The Rust CLI exposes a dedicated `--sandbox` (`-s`) flag that lets you pick the sandbox policy **without** having to reach for the generic `-c/--config` option:

```shell
# Run Beacon with the default, read-only sandbox
codex --sandbox read-only

# Allow the agent to write within the current workspace while still blocking network access
codex --sandbox workspace-write

# Danger! Disable sandboxing entirely (only do this if you are already running in a container or other isolated env)
codex --sandbox danger-full-access
```

The same setting can be persisted in `~/.codex/config.toml` via the top-level `sandbox_mode = "MODE"` key, e.g. `sandbox_mode = "workspace-write"`.

## Code Organization

This folder is the root of a Cargo workspace. It contains quite a bit of experimental code, but here are the key crates:

- [`core/`](./core) contains the business logic for Beacon. Ultimately, we hope this to be a library crate that is generally useful for building other Rust/native applications that use Beacon.
- [`exec/`](./exec) "headless" CLI for use in automation.
- [`tui/`](./tui) CLI that launches a fullscreen TUI built with [Ratatui](https://ratatui.rs/).
- [`cli/`](./cli) CLI multitool that provides the aforementioned CLIs via subcommands.
