---
title: AGENTS.md discovery
---

# AGENTS.md discovery

This page documents how this repo’s CLIs discover and merge project instructions from `AGENTS.md`-style files.

Why this matters:

- It defines **which instructions are in scope** for a session.
- It prevents “mystery behavior” when a nested directory has different guidance than the repo root.
- It helps maintainers know **where to put instructions** so they apply to the right subtree.

## Terminology

- **Global/personal instructions**: instructions stored under the CLI’s home directory (e.g. `~/.code/AGENTS.md` or `~/.codex/AGENTS.md`).
- **Project instructions**: instructions stored inside a Git repository and discovered from the repo root down to the current working directory.
- **Git root**: the closest ancestor directory (walking upward) that contains a `.git` directory or file.

## Summary (most common behavior)

- Start from the current working directory (`cwd`).
- Find the Git root by walking upwards until `.git` is found.
- Collect one instruction file per directory from Git root → `cwd` (inclusive).
- Concatenate them **in that order** (root first, deepest last).
- Never walk past the Git root.

## Behavior by workspace (important: this repo has two Rust workspaces)

### `code-rs/` (Every Code)

Implemented in `code-rs/core/src/project_doc.rs`.

- **Project doc filenames**:
  - `AGENTS.md` (project instructions)
  - `AUTO_AGENTS.md` (Auto Drive-specific project instructions)
- **Global instructions**:
  - Loaded from `CODE_HOME/AGENTS.md` (default: `~/.code/AGENTS.md`).
  - Falls back to legacy `CODEX_HOME/AGENTS.md` (default: `~/.codex/AGENTS.md`) for backwards compatibility.
- **Overrides**:
  - `AGENTS.override.md` is **not** recognized in `code-rs/` today.
- **Size limits**:
  - The maximum bytes embedded from project docs is controlled by `project_doc_max_bytes` (default: 32 KiB).
  - When the limit is reached, later files are truncated or skipped.

### `codex-rs/` (upstream mirror)

Implemented in `codex-rs/core/src/project_doc.rs`.

- Supports:
  - `AGENTS.md` (project instructions)
  - `AGENTS.override.md` (directory-level “replace inherited instructions” override)
  - `project_doc_fallback_filenames` (additional filenames to try when `AGENTS.md` is missing)
- May append a runtime-only **Skills** section after project docs when the Skills feature is enabled.

## Practical guidance

- Put repo-wide guidance in the repo root `AGENTS.md`.
- Put narrow, subsystem-specific guidance in a nested directory’s `AGENTS.md` so it only applies to that subtree.
- Avoid duplicating the same instructions in multiple levels; prefer “general at root, specific deeper”.
- Keep instructions short and stable; long documents consume context and can reduce model quality.

