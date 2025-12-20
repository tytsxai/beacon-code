---
title: Environment variables
---

# Environment variables

This page lists notable environment variables supported by this repository. It focuses on variables that materially affect security, permissions, persistence, or user-facing behavior. Many CI/test-only variables also exist; treat those as internal unless documented here.

If you only need the basics (auth + config), see `docs/command-reference.md`.

## Conventions

- Unless stated otherwise, values are treated as strings; booleans typically accept `"1"` or `"true"`.
- Variables are read by the Rust CLI (`code-rs/`) unless stated otherwise.
- `CODE_HOME` is the primary home directory for Beacon Code.

## Authentication / API access

- `OPENAI_API_KEY`
  - Purpose: authenticate against OpenAI APIs (pay-as-you-go).
  - Risk: grants API access; treat as a secret.
  - Docs: `docs/authentication.md`

- `OPENAI_BASE_URL`
  - Purpose: override the OpenAI API base URL (for proxies/mirrors).
  - Risk: misconfiguration can send traffic to an unintended endpoint.
  - Docs: `docs/config.md` (model providers)

- `BEACON_API_KEY`
  - Purpose: override the API key for `code exec` (non-interactive mode).
  - Notes: `CODEX_API_KEY` is a legacy alias kept for compatibility.

## Config / state directories

- `CODE_HOME`
  - Purpose: override the base directory for config, auth, history, logs, etc.
  - Default: `~/.code`

## Build / binary selection

- `CODE_VERSION`
  - Purpose: override the version string embedded in the Rust binary at build time.
  - Default: `CARGO_PKG_VERSION` when not set.
  - Notes: intended for CI/release builds; end users typically do not need to set this.
  - Code: `code-rs/code-version/build.rs`

- `CODE_BINARY_PATH`
  - Purpose: override which native binary the wrapper uses (or communicates to child processes).
  - Risk: pointing this at an unexpected binary can run untrusted code.
  - Code: `code-rs/core/src/agent_tool.rs`

## Cleanup / retention (high risk)

These variables affect deletion of files under `CODE_HOME` (sessions, worktrees, logs). Use with care.

- `CODE_CLEANUP_DISABLE`
  - Purpose: disable housekeeping entirely.
  - Values: `"1"`, `"true"`, `"on"`, `"yes"` disable.
  - Code: `code-rs/core/src/housekeeping.rs`

- `CODE_CLEANUP_SESSION_RETENTION_DAYS`
  - Purpose: how many days of sessions to retain (older are pruned).
  - Default: 7

- `CODE_CLEANUP_WORKTREE_RETENTION_DAYS`
  - Purpose: how many days of old worktrees to retain.
  - Default: 3

- `CODE_CLEANUP_LOG_RETENTION_DAYS`
  - Purpose: how many days of logs to retain.
  - Default: 14

- `CODE_CLEANUP_LOG_MAX_BYTES`
  - Purpose: optional max size for log retention/truncation.
  - Default: 50 MiB

- `CODE_CLEANUP_LOG_TRUNCATE_MIN_AGE_MINUTES`
  - Purpose: do not truncate very recent logs.
  - Default: 10 minutes

- `CODE_CLEANUP_MIN_INTERVAL_HOURS`
  - Purpose: minimum interval between housekeeping runs.
  - Default: 6 hours

## TUI rendering / UX

- `CODE_DISABLE_THEME_AUTODETECT`
  - Purpose: disables terminal theme autodetection.
  - Code: `code-rs/tui/src/lib.rs`

- `CODE_ENABLE_FOCUS`, `CODE_DISABLE_FOCUS`
  - Purpose: control focus tracking (some terminals do not support it reliably).
  - Code: `code-rs/tui/src/tui.rs`

- `CODE_FORCE_TRUECOLOR`
  - Purpose: force truecolor rendering.
  - Code: `code-rs/tui/src/theme.rs`

- `CODE_FORCE_ANSI256`
  - Purpose: force ANSI-256 palette fallback.
  - Code: `code-rs/tui/src/theme.rs`

- `CODE_FORCE_FULL_BG_PAINT`
  - Purpose: force full background painting in the TUI (workaround for terminal rendering quirks).
  - Code: `code-rs/tui/src/tui.rs`

- `CODE_TUI_REDUCED_MOTION`
  - Purpose: reduce or disable certain animations (accessibility).
  - Code: `code-rs/tui/src/chatwidget.rs`

## Agents / cloud (advanced)

- `CODE_ENABLE_CLOUD_AGENT_MODEL`
  - Purpose: enable the cloud-hosted agent model variant in the default agent roster.
  - Risk: enabling it can add latency and can route agent traffic differently than local models.
  - Code: `code-rs/core/src/agent_defaults.rs`

## Hooks / project commands (advanced)

These are exported to hook subprocesses so they can understand what triggered them. They are not intended to be set by users directly.

- `CODE_HOOK_EVENT`, `CODE_HOOK_TRIGGER`, `CODE_HOOK_CALL_ID`, `CODE_HOOK_SUB_ID`, `CODE_HOOK_INDEX`, `CODE_HOOK_PAYLOAD`, `CODE_HOOK_NAME`, `CODE_HOOK_SOURCE_CALL_ID`
  - Purpose: describe the hook invocation.
  - Code: `code-rs/core/src/codex.rs`

- `CODE_PROJECT_COMMAND_NAME`, `CODE_PROJECT_COMMAND_DESCRIPTION`, `CODE_SESSION_CWD`
  - Purpose: describe the invoked project command and session working directory.
  - Code: `code-rs/core/src/codex.rs`

## Packaging / install (Node wrapper)

These are used by the Node wrapper package.

- `CODE_FORCE_PREINSTALL`, `CODE_SKIP_PREINSTALL`
  - Purpose: control whether the preinstall checks run (mainly for CI/debugging).
  - Risk: skipping checks can hide install problems; forcing can make installs fail on unsupported platforms.

- `CODE_POSTINSTALL_DRY_RUN`
  - Purpose: skip actual postinstall actions (tests/dry run).

- `CODE_RUNTIME_POSTINSTALL`
  - Purpose: allow running runtime postinstall bootstrap when the native binary is missing.

## Diagnostics (internal / debug)

These are primarily used for debugging or tests; they are listed here because they can materially affect behavior/output.

- `CODE_TUI_TEST_MODE`
  - Purpose: force deterministic TUI behavior for tests/snapshots.
  - Code: `code-rs/tui/src/chatwidget.rs`

- `CODE_BUFFER_DIFF_TRACE_CELLS`, `CODE_BUFFER_DIFF_METRICS`
  - Purpose: enable additional TUI diff diagnostics (verbose output/logging).
  - Code: `code-rs/tui/src/chatwidget.rs`

- `CODE_DISABLE_AUTO_DRIVE_DIAGNOSTICS`
  - Purpose: reduce noisy diagnostics emitted by Auto Drive UI paths.
  - Code: `code-rs/tui/src/chatwidget.rs`
