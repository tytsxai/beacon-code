# Production Readiness Checklist

This repository ships a **local CLI** (primary) and an optional stdio JSON-RPC
gateway (`code-app-server`) used by MCP clients. There is **no database** in
this project; state lives under `$CODE_HOME` (default `~/.code`).

## 1. Pre-Deployment Verification
- [ ] **CI Pipeline**: The target commit passed JS (format + dependency audit),
      Rust (fmt/clippy), tests, and `cargo audit` checks.
    - [ ] If CI does not run JS auditing, run `pnpm audit --prod` (or your SCA tool)
          before release.
- [ ] **Readiness sweep**: run `bash scripts/prod_ready_sweep.sh` and confirm the
      summary reports `result: ok`.
- [ ] **Config sanity**:
    - [ ] `beacon doctor` (or `code doctor`) completes without errors.
    - [ ] `config.toml` and `managed_config.toml` (if used) are valid and in the
          expected location (`$CODE_HOME` or `/etc/code/managed_config.toml` on Unix).
- [ ] **Security baseline (required)**:
    - [ ] Enforce a managed config baseline (recommended for production).
          Use `docs/ops/managed_config.production.toml` as a starting point:
      ```toml
      # /etc/code/managed_config.toml (Unix) or $CODE_HOME/managed_config.toml
      approval_policy = "untrusted"
      sandbox_mode = "workspace-write"
      auto_upgrade_enabled = false
      forced_login_method = "chatgpt" # or "api"

      [sandbox_workspace_write]
      network_access = false
      ```
    - [ ] For long-running services, set `CODE_SECURE_MODE=1` in the environment.
    - [ ] Ensure `CODE_HOME` is owned by the service user and not group/world
          readable (e.g., `chmod 700` on the directory or `UMask=0077` in your
          service manager).
- [ ] **Runtime guardrails**:
    - [ ] Pin `auto_upgrade_enabled` explicitly (recommended: `false` for production).
    - [ ] If Auto Drive is enabled, set `[auto_drive]` limits for `token_budget`,
          `turn_limit`, and `duration_limit_seconds`, plus `parallel_instances`.
- [ ] **Release Inputs**:
    - [ ] All platform binaries were built by `rust-release` (Linux/macOS/Windows).
    - [ ] `beacon-cli/scripts/build_npm_package.py` generated `checksums.json`
          from the release binaries (npm installs verify these checksums).
- [ ] **Secrets & auth**:
    - [ ] `OPENAI_API_KEY` / ChatGPT auth is stored in the intended secret store.
    - [ ] Avoid committing secrets into `config.toml` or `managed_config.toml`.
- [ ] **State & Backup**:
    - [ ] If you need to preserve state, back up at least:
      - `$CODE_HOME/config.toml`, plus `/etc/code/managed_config.toml` (if used on Unix)
      - `$CODE_HOME/auth.json`
      - `$CODE_HOME/auth_accounts.json`
      - `$CODE_HOME/history.jsonl`
      - `$CODE_HOME/sessions/`
      - `$CODE_HOME/log/` and `$CODE_HOME/logs/`
    - [ ] Optional (if you rely on them): `$CODE_HOME/.env`, `$CODE_HOME/prompts/`,
          `$CODE_HOME/rules/`, `cache.json`, `$CODE_HOME/checkpoints/`, or `working/` worktrees.
    - [ ] Use `scripts/code-home-backup.sh` to capture/restore snapshots:
      - `scripts/code-home-backup.sh backup --out /path/backup.tgz`
      - `scripts/code-home-backup.sh restore --in /path/backup.tgz --code-home /tmp/code-restore`
      - Verify `code --version` and `code resume --last` after restore.
      - Add `--include-worktrees` if you need to preserve `working/` worktrees.
    - [ ] Verify backup archives before rollout:
      - `scripts/ops/verify-backup.sh --in /path/backup.tgz`
      - Add `--require-worktrees` if you rely on `working/` worktrees.
    - [ ] Housekeeping can prune sessions/worktrees/logs. Defaults: sessions 7d,
          worktrees 3d, logs 14d, log size 50 MiB. Set `CODE_CLEANUP_*` (or
          `CODE_CLEANUP_DISABLE=1`) to meet retention requirements.
    - [ ] Verify `CODE_HOME` permissions (service user only):
      - `scripts/ops/verify-code-home.sh --code-home /var/lib/code`
    - [ ] Review `docs/ops/production.env.example` for a baseline env template.

## 1.5 Functional & Failure-Mode Validation
- [ ] **Auth happy path**: login succeeds (ChatGPT or API key), CLI completes a
      short session, and history writes under `$CODE_HOME`.
- [ ] **Auth failure path**: invalid/expired credentials return a clear error
      and do not crash the CLI or app-server.
- [ ] **Sandbox enforcement**: read-only mode blocks writes, and workspace-write
      blocks network when `network_access = false`.
- [ ] **State isolation**: starting a second instance with the same `CODE_HOME`
      fails fast with a clear error; parallel instances use distinct `CODE_HOME`.
- [ ] **Auto Drive guardrails** (if enabled): token/turn/time limits halt runs
      predictably and can be resumed safely.

## 2. Deployment

### 2.1 CLI (npm/Homebrew/archives)
- [ ] **npm**: `npm install -g @tytsxai/beacon-code@<version>`
- [ ] **Homebrew**: `brew install code` (if using the formula)
- [ ] **Smoke checks**:
  - `beacon --version`
  - `beacon doctor`
  - `scripts/ops/healthcheck-cli.sh` (optional automation)
- [ ] **Container**: `beacon-cli/Dockerfile` is supported for container builds.
  It sets `BEACON_UNSAFE_ALLOW_NO_SANDBOX=1` because the container is expected
  to provide its own isolation.
  - Enforce network isolation with `beacon-cli/scripts/init_firewall.sh` (or
    use `beacon-cli/scripts/run_in_container.sh`).
  - Required Docker flags: `--cap-add=NET_ADMIN --cap-add=NET_RAW`.
  - Provide allowed domains via `/etc/beacon-code/allowed_domains.txt` or
    `OPENAI_ALLOWED_DOMAINS`.
  - Ensure `CODE_HOME` is on a persistent volume if state must survive restarts.
  - IPv6 must be controlled: ensure `ip6tables` is available or disable IPv6 in
    the container. Do not rely on IPv4-only rules.
  - Do not run the container without an explicit network policy or firewall.
  - Runtime guard: the CLI refuses to start when sandboxing is disabled unless
    `/etc/beacon-code/firewall.ready` exists. Create it by running
    `beacon-cli/scripts/init_firewall.sh` (or `run_in_container.sh`).
    Emergency bypass: `BEACON_ALLOW_UNSAFE_NO_FIREWALL=1` (not recommended).

### 2.2 App-server (optional)
- [ ] **Binary name**: `code-app-server` (service name can be anything, e.g.
  `beacon-app-server`).
- [ ] **Runtime**: JSON-RPC over stdin/stdout; run under a supervisor that keeps
  stdin open.
- [ ] **Access control**: treat stdin/stdout as privileged. Do not expose this
  service directly on a network socket; the app-server has no built-in auth. If
  you wrap it with HTTP, add authn/z and restrict clients to trusted callers.
- [ ] **Liveness**: treat a successful `Initialize` response as a health check.
  - Optional script: `scripts/ops/healthcheck-app-server.sh /path/to/socket --timeout 5`.
- [ ] **Runbook**: follow `docs/ops/app-server-runbook.md` for supervisor and
  socket activation examples.
- [ ] **Environment**:
  - `RUST_LOG=info` (or your preferred filter)
  - `CODE_SECURE_MODE=1` (hardening; `CODEX_SECURE_MODE` is legacy)
- [ ] **State isolation**: run one app-server per `CODE_HOME`; use distinct
  `CODE_HOME` paths if you need parallel instances.

## 3. Rollback Procedure
- [ ] **Trigger**: Crash loop, critical regression, or unacceptable error rate.
- [ ] **CLI rollback**:
  - `npm install -g @tytsxai/beacon-code@<previous>`
  - For Homebrew: reinstall the prior version if applicable.
- [ ] **App-server rollback**:
  - Redeploy the previous `code-app-server` binary or image.
  - Restore `$CODE_HOME` from the last known good backup if state loss is part
    of the incident.

## 4. Observability
- [ ] **CLI logs**:
  - `~/.code/log/code-tui.log` (general)
  - `~/.code/logs/critical.log` (errors)
- [ ] **App-server logs**: stderr (capture via systemd/Docker/K8s).
- [ ] **OTEL**: configure exporter in `config.toml` for telemetry log export and
  set `environment = "production"`. Keep `log_user_prompt = false` unless your
  compliance rules allow prompt capture. Metrics are not emitted by default;
  use log-based metrics unless you add instrumentation.
- [ ] **Retention**: confirm housekeeping is enabled and `CODE_CLEANUP_*`
  matches your ops retention policy.

## 5. Alerting (only if running app-server)
- **Critical**: any `"Panic occurred"` log line.
- **Critical**: crash loop (>3 restarts in 10 minutes).
- If you wrap stdio JSON-RPC with an HTTP gateway, use standard 5xx/latency
  alerts for that gateway (queries in Section 7.3).

## 6. On-Call Procedures

### 6.1 Escalation Path
1. **L1 - On-Call Engineer** (responds within 15 minutes)
   - Triage alert severity
   - Check logs and metrics
   - Attempt standard remediation (restart, rollback)

2. **L2 - Team Lead** (escalate after 30 minutes if unresolved)
   - Review incident context
   - Coordinate with other teams if needed
   - Approve emergency changes

3. **L3 - Engineering Manager** (escalate for critical outages >1 hour)
   - Engage vendor support if applicable
   - Coordinate external communication
   - Post-incident review planning

### 6.2 Initial Triage Steps
1. **Acknowledge Alert** (within 5 minutes)
   - Mark incident in PagerDuty/Opsgenie
   - Post in #incidents Slack channel

2. **Gather Context** (within 15 minutes)
   ```bash
   # Check recent deployments (if applicable)
   kubectl rollout history deployment/code-app-server -n production

   # Review logs for errors
   kubectl logs -n production -l app=code-app-server --tail=500 | grep -i error

   # Check resource usage
   kubectl top pods -n production -l app=code-app-server
   ```

3. **Decide Action** (within 20 minutes)
   - If recent deployment: rollback (see Section 3)
   - If resource exhaustion: scale up or restart
   - If unknown: escalate to L2

### 6.3 Communication Templates

**Initial Incident Post** (Slack #incidents):
```
INCIDENT: [CRITICAL/WARNING] - [Brief Description]
- Time: [HH:MM UTC]
- Impact: [% users affected / services down]
- Status: Investigating
- Owner: @engineer-name
- Dashboard: [link]
```

**Resolution Post**:
```
RESOLVED: [Brief Description]
- Duration: [X minutes]
- Root Cause: [1-2 sentences]
- Resolution: [Action taken]
- Follow-up: [Ticket link for post-mortem]
```

## 7. Monitoring Configuration

### 7.1 OTLP Endpoint Setup

Configure OTEL in `$CODE_HOME/config.toml`:

```toml
[otel]
environment = "production"
log_user_prompt = false

[otel.exporter."otlp-grpc"]
endpoint = "https://otlp.your-monitoring.com:4317"

[otel.exporter."otlp-grpc".headers]
x-api-key = "your-api-key-here"
```

### 7.2 Log Aggregation Setup

**Structured Logging Configuration**:
```bash
export RUST_LOG="info,code_core=debug,code_app_server=debug"
```

**Log Collection** (Fluent Bit example):
```yaml
# fluent-bit.conf
[INPUT]
    Name              tail
    Path              /var/log/containers/code-app-server*.log
    Parser            docker
    Tag               kube.code-app-server
    Refresh_Interval  5

[FILTER]
    Name                kubernetes
    Match               kube.*
    Kube_URL            https://kubernetes.default.svc:443
    Merge_Log           On
    Keep_Log            Off

[OUTPUT]
    Name  es
    Match kube.code-app-server
    Host  elasticsearch.monitoring.svc
    Port  9200
    Index beacon-logs
```

### 7.3 Dashboard Recommendations (HTTP wrapper only)

If you expose an HTTP gateway around `code-app-server`, use standard request
rate/error/latency panels for the gateway. Example Prometheus queries:

- **Error rate**:
  `sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m]))`
- **P99 latency**:
  `histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))`
