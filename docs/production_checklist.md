# Production Readiness Checklist

This repository ships a **local CLI** (primary) and an optional stdio JSON-RPC
gateway (`code-app-server`) used by MCP clients. There is **no database** in
this project; state lives under `$CODE_HOME` (default `~/.code`).

## 1. Pre-Deployment Verification
- [ ] **CI Pipeline**: The target commit passed JS (format), Rust (fmt/clippy),
      tests, and `cargo audit` checks.
- [ ] **Release Inputs**:
    - [ ] All platform binaries were built by `rust-release` (Linux/macOS/Windows).
    - [ ] `beacon-cli/scripts/build_npm_package.py` generated `checksums.json`
          from the release binaries (npm installs verify these checksums).
- [ ] **State & Backup**:
    - [ ] If you need to preserve auth/session history, back up
          `$CODE_HOME/auth.json`, `$CODE_HOME/sessions/`, `$CODE_HOME/log/`, and
          `$CODE_HOME/logs/`.

## 2. Deployment

### 2.1 CLI (npm/Homebrew/archives)
- [ ] **npm**: `npm install -g @tytsxai/beacon-code@<version>`
- [ ] **Homebrew**: `brew install code` (if using the formula)
- [ ] **Smoke checks**:
  - `beacon --version`
  - `beacon doctor`
- [ ] **Container**: `beacon-cli/Dockerfile` is supported for container builds.
  It sets `BEACON_UNSAFE_ALLOW_NO_SANDBOX=1` because the container is expected
  to provide its own isolation.

### 2.2 App-server (optional)
- [ ] **Binary name**: `code-app-server` (service name can be anything, e.g.
  `beacon-app-server`).
- [ ] **Runtime**: JSON-RPC over stdin/stdout; run under a supervisor that keeps
  stdin open.
- [ ] **Liveness**: treat a successful `Initialize` response as a health check.
- [ ] **Environment**:
  - `RUST_LOG=info` (or your preferred filter)
  - `CODE_SECURE_MODE=1` (hardening; `CODEX_SECURE_MODE` is legacy)

## 3. Rollback Procedure
- [ ] **Trigger**: Crash loop, critical regression, or unacceptable error rate.
- [ ] **CLI rollback**:
  - `npm install -g @tytsxai/beacon-code@<previous>`
  - For Homebrew: reinstall the prior version if applicable.
- [ ] **App-server rollback**:
  - Redeploy the previous `code-app-server` binary or image.

## 4. Observability
- [ ] **CLI logs**:
  - `~/.code/log/code-tui.log` (general)
  - `~/.code/logs/critical.log` (errors)
- [ ] **App-server logs**: stderr (capture via systemd/Docker/K8s).
- [ ] **OTEL**: configure exporter in `config.toml` when you need metrics.

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
