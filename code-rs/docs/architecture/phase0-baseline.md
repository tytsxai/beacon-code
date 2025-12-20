# Phase 0 Baseline – Architecture Survey (September 20, 2025)

This note captures the current layout of the `code-rs` workspace before any
phase-one refactors. It should help us reason about the execution pipeline and
identify the modules that will be split up in later phases.

## Workspace Layout

- **Core execution**
  - `code-core`: orchestration crate exposing `Codex` (Beacon engine),
    conversation management, exec tooling, confirm-guard policy, safety
    wrappers, and project/plan utilities. Most business logic lives here today.
  - `code-exec`: headsless CLI that streams protocol events to stdout/human or
    JSON renderers.
  - `code-cli`: thin binary that wires auth/config to the TUI.
- **UI / presentation**
  - `code-tui`: Ratatui-based interface; owns chat widget, history cells,
    bottom pane, onboarding, streaming controller, and layout logic.
- **Shared utilities**
  - `code-common`: shared config summaries, presets, timers, CLI argument
    parsing helpers.
  - `code-ansi-escape`, `code-browser`, `code-file-search`, `code-login`,
    `code-apply-patch`, etc. provide specialized services re-exported by
    `code-core`.
- **Model/control plane**
  - `code-mcp-*`: client/server/test fixtures for Model Context Protocol.
  - `code-protocol`, `code-protocol-ts`: protocol definitions consumed by
    Rust and TypeScript surfaces.
- **Ancillary crates**
  - Tooling (`code-ollama`, `code-linux-sandbox`, `code-browser`), smoke
    tests, and TS bindings (`code-browser`, `code-arg0`, etc.).

## Core Command Pipeline (Today)

1. **Conversation spawn** (`code-core::ConversationManager`)
   - Creates `Codex` (Beacon engine) via `Codex::spawn`, yielding an async event
     stream.
   - Produces `SessionConfiguredEvent` and registers the conversation in a
     shared `RwLock<HashMap<ConversationId, Arc<BeaconConversation>>>`.
2. **Event loop** (`code-core::Codex` & `BeaconConversation`)
   - `Codex::next_event` polls the MCP transport and channel fan-out to yield
     `EventMsg` values.
   - `code-core/src/codex.rs` manages confirm guards, tool dispatch, browser
     snapshots, streaming output buffers, and local command execution.
3. **Frontend consumption**
   - `code-tui::chatwidget` subscribes through `AppEventSender` and mutates a
     `HistoryRenderState` alongside UI layout caches. Rendering merges state
     with Ratatui widgets in `history_cell`, `diff_render`, and friends.
   - `code-exec` consumes the same events for non-interactive sessions.

## Identified Monoliths

- `code-core/src/codex.rs` (~3k LOC) interleaves policy checks, tool
  invocation, browser hooks, and response assembly. Later phases will split it
  into smaller services (`ConfirmGuard`, `ToolBroker`, `ResponseAssembler`, etc.).
- `code-tui/src/chatwidget.rs` (~3k LOC) couples event wiring, command state,
  rendering hints, and approval flows. Target is a reducer-style `ChatState`
  plus feature-specific controllers.
- `code-tui/src/history_cell.rs` implements a trait object hierarchy with
  manual downcasts. Moving to a typed enum model will simplify rendering and
  caching.

## Current Testing & Tooling Gaps

- No lightweight integration test ensures event ordering invariants for the
  strict TUI history stream (per the TUI contract).
- `./build-fast.sh` is the single required check; results are not currently
  captured in documentation for regression comparison.

## Baseline Metrics

- `./build-fast.sh` (dev-fast) completed successfully on September 20, 2025 in
  14.87s producing `target/dev-fast/code` (hash
  `4162f125c8a0afb8f513e6e6a98ba57035aa2fb39959295c2567ec4699f0e965`).

## Next Actions for Phase 0

1. Record baseline `./build-fast.sh` duration and success in this folder.
2. Introduce a smoke test under `code-core/tests/` that drives a short mock
   conversation and asserts strictly ordered `EventMsg` IDs.
3. Evaluate adding a TUI snapshot test harness once the chat state reducer is
   available (tracked for later phases).
