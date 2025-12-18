---
title: High throughput Auto Drive
---

# High throughput Auto Drive (experimental)

This note summarizes the “high throughput” components used by Auto Drive in the `code-rs/` workspace. It is intended for maintainers who need to understand the concurrency model, persistence points, and where “implicit rules” live.

## Where it lives

- Core logic: `code-rs/code-auto-drive-core/src/`
- Key modules:
  - `session_pool.rs`: session pool sizing, health checks, backpressure
  - `parallel_execution.rs`: parallel agent execution with concurrency limits
  - `task_pipeline.rs` + `role_channel.rs`: staged task execution and message passing
  - `progress_log.rs` + `progress.rs`: on-disk progress reporting (when enabled)
  - `backlog.rs` + `selective_tests.rs`: test selection and verification recording
  - `checkpoint.rs`: checkpoint persistence (when enabled)
  - `audit.rs`: audit logging (when enabled)

## What it is (conceptually)

High throughput mode is a coordination strategy that trades latency for throughput by:

- keeping a pool of ready-to-run sessions (instead of creating sessions on demand),
- executing multiple agents concurrently (bounded by explicit limits),
- funneling results through a staged pipeline so the coordinator can decide whether to proceed, retry, or stop.

## Configuration surface

This repo’s examples describe the following knobs (exact keys are defined in config types under `code-rs/core/src/config_types.rs` and the Auto Drive coordinator):

- `max_concurrent_agents`: cap parallel agent executions
- “high throughput” pool settings (session min/max, scale thresholds, backpressure multiplier)
- `checkpoint_enabled` / `checkpoint_dir` / `checkpoint_interval`
- `audit_enabled` / `audit_path`
- `token_budget`, `turn_limit`, `duration_limit_seconds`

If you change defaults or rename keys, update:

- `docs/architecture.md`
- `config.toml.example`
- `docs/example-config.md` and `docs/zh/config-examples.md` (if the change is user-facing)

## Operational invariants (high-risk, easy to break)

- **Concurrency must be bounded**: parallelism is explicitly limited (e.g. via a semaphore). Removing the cap will cause resource blowups and make UI behavior flaky.
- **Backpressure is intentional**: when queues grow near the pool limit, the system emits warnings rather than continuing to enqueue forever.
- **Persistence must be atomic**: checkpoint/progress/audit outputs are written in a way that tolerates crashes. Avoid writing partial files without atomic replace.
- **Session reuse is stateful**: pooled sessions carry state (env, cwd, tool caches). Pool health checks and “stuck” detection are not optional niceties; they protect correctness.

