# Production Readiness Checklist

## 1. Pre-Deployment Verification
- [ ] **CI Pipeline**: Ensure the latest commit has passed all CI checks (Lint, Test, Build).
- [ ] **Security Audit**:
    - [ ] `cargo audit` returns no high-severity vulnerabilities.
    - [ ] No secrets committed in source code (check `.env` and config files).
- [ ] **Data Migration**:
    - [ ] DB migrations tested on staging/pre-prod.
    - [ ] Backup created before applying migrations.

## 2. Deployment
- [ ] **Container Image**: Built with `release` profile (optimized, stripped symbols).
- [ ] **Environment Variables**:
    - [ ] `RUST_LOG=info` (or structured JSON logging config).
    - [ ] `CODE_SECURE_MODE=1` (if applicable for hardening; `CODEX_SECURE_MODE` is legacy).
- [ ] **Service Mode**: `codex-app-server` speaks JSON-RPC over stdin/stdout; run it under a supervisor that keeps stdin open, and treat liveness as process + responsiveness to a basic request (e.g., Initialize).
- [ ] **Health Check**: If you expose an HTTP wrapper, `/health` returns 200 OK.

## 3. Rollback Procedure
- [ ] **Trigger**: High error rate (>1%), crash loop, or critical functional regression.
- [ ] **Steps**:
    1.  Revert to the previous stable Docker image tag.
    2.  If DB migration occurred and is backward-incompatible, run the down-migration script (carefully!).
    3.  Restart services.
    4.  Verify health check.

### 3.1 Docker Rollback Commands
```bash
# List recent image tags
docker images | grep codex-app-server | head -5

# Rollback to previous stable tag
docker stop codex-app-server
docker rm codex-app-server
docker run -d --name codex-app-server \
  --restart unless-stopped \
  -e RUST_LOG=info \
  -e CODE_SECURE_MODE=1 \
  your-registry/codex-app-server:v1.2.3-stable

# Verify rollback
docker logs -f codex-app-server
curl http://localhost:8080/health
```

### 3.2 Kubernetes Rollback Commands
```bash
# Check deployment history
kubectl rollout history deployment/codex-app-server -n production

# Rollback to previous revision
kubectl rollout undo deployment/codex-app-server -n production

# Rollback to specific revision
kubectl rollout undo deployment/codex-app-server -n production --to-revision=5

# Monitor rollback status
kubectl rollout status deployment/codex-app-server -n production

# Verify pods are healthy
kubectl get pods -n production -l app=codex-app-server
kubectl logs -n production -l app=codex-app-server --tail=100
```

### 3.3 Database Rollback Script Template
```sql
-- down_migration_v2.sql
-- CAUTION: Test on staging first!

BEGIN;

-- Example: Revert column addition
ALTER TABLE sessions DROP COLUMN IF EXISTS new_feature_flag;

-- Example: Restore old schema
ALTER TABLE users RENAME COLUMN email_v2 TO email;

-- Verify row count before commit
SELECT COUNT(*) FROM sessions;
SELECT COUNT(*) FROM users;

-- If counts look correct, commit; otherwise ROLLBACK manually
COMMIT;
```

Apply with:
```bash
# Backup first
pg_dump -h localhost -U postgres -d codex_db > backup_before_rollback.sql

# Apply down migration
psql -h localhost -U postgres -d codex_db -f down_migration_v2.sql

# Verify
psql -h localhost -U postgres -d codex_db -c "\d sessions"
```

## 4. Post-Deployment Observability
- [ ] **Logs**: Verify `app-server` logs are flowing to the aggregation system.
- [ ] **Errors**: Monitor for "Panic occurred" or unhandled exceptions in stderr.
- [ ] **Metrics**: Check CPU/Memory usage is within baseline.

## 5. Alert Thresholds

### 5.1 Error Rate Alerts
- **Critical**: Error rate >1% over 5-minute window
  - Action: Page on-call engineer immediately
  - Query: `sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m])) > 0.01`

- **Warning**: Error rate >0.5% over 10-minute window
  - Action: Notify team channel
  - Query: `sum(rate(http_requests_total{status=~"5.."}[10m])) / sum(rate(http_requests_total[10m])) > 0.005`

### 5.2 Latency Alerts
- **Critical**: P99 latency >5s for 5 minutes
  - Action: Page on-call engineer
  - Query: `histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 5`

- **Warning**: P95 latency >2s for 10 minutes
  - Action: Notify team channel
  - Query: `histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[10m])) > 2`

### 5.3 Resource Alerts
- **Critical**: Memory usage >90% for 5 minutes
  - Action: Page on-call engineer (potential OOM)
  - Query: `(container_memory_usage_bytes / container_spec_memory_limit_bytes) > 0.9`

- **Warning**: CPU usage >80% for 15 minutes
  - Action: Notify team channel
  - Query: `rate(container_cpu_usage_seconds_total[5m]) > 0.8`

### 5.4 Panic Detection
- **Critical**: Any panic in logs
  - Action: Page on-call engineer immediately
  - Query: `count_over_time({job="codex-app-server"} |= "Panic occurred"[5m]) > 0`

- **Critical**: Crash loop (>3 restarts in 10 minutes)
  - Action: Page on-call engineer
  - Query: `rate(kube_pod_container_status_restarts_total{pod=~"codex-app-server.*"}[10m]) > 0.3`

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

2. **Assess Impact** (within 10 minutes)
   - Check error rate dashboard
   - Verify affected services/regions
   - Estimate user impact percentage

3. **Gather Context** (within 15 minutes)
   ```bash
   # Check recent deployments
   kubectl rollout history deployment/codex-app-server -n production

   # Review logs for errors
   kubectl logs -n production -l app=codex-app-server --tail=500 | grep -i error

   # Check resource usage
   kubectl top pods -n production -l app=codex-app-server

   # Verify dependencies
   curl http://internal-api/health
   ```

4. **Decide Action** (within 20 minutes)
   - If recent deployment: Rollback (see Section 3)
   - If resource exhaustion: Scale up or restart
   - If external dependency: Engage vendor
   - If unknown: Escalate to L2

### 6.3 Communication Templates

**Initial Incident Post** (Slack #incidents):
```
🚨 INCIDENT: [CRITICAL/WARNING] - [Brief Description]
- Time: [HH:MM UTC]
- Impact: [% users affected / services down]
- Status: Investigating
- Owner: @engineer-name
- Dashboard: [link]
```

**Resolution Post**:
```
✅ RESOLVED: [Brief Description]
- Duration: [X minutes]
- Root Cause: [1-2 sentences]
- Resolution: [Action taken]
- Follow-up: [Ticket link for post-mortem]
```

## 7. Monitoring Configuration

### 7.1 OTLP Endpoint Setup

**Environment Variables**:
```bash
# gRPC endpoint (recommended for production)
export OTEL_EXPORTER_OTLP_ENDPOINT="https://otlp.your-monitoring.com:4317"
export OTEL_EXPORTER_OTLP_PROTOCOL="grpc"
export OTEL_EXPORTER_OTLP_HEADERS="x-api-key=your-api-key-here"

# HTTP endpoint (alternative)
export OTEL_EXPORTER_OTLP_ENDPOINT="https://otlp.your-monitoring.com:4318"
export OTEL_EXPORTER_OTLP_PROTOCOL="http/protobuf"
export OTEL_EXPORTER_OTLP_HEADERS="Authorization=Bearer your-token-here"

# Service identification
export OTEL_SERVICE_NAME="codex-app-server"
export OTEL_SERVICE_VERSION="1.2.3"
export OTEL_ENVIRONMENT="production"
```

**Rust Code Configuration** (already supported in `otel/src/config.rs`):
```rust
use codex_otel::config::{OtelSettings, OtelExporter, OtelHttpProtocol};
use std::collections::HashMap;

let mut headers = HashMap::new();
headers.insert("x-api-key".to_string(), "your-api-key".to_string());

let settings = OtelSettings {
    environment: "production".to_string(),
    service_name: "codex-app-server".to_string(),
    service_version: env!("CARGO_PKG_VERSION").to_string(),
    code_home: PathBuf::from("/app"),
    exporter: OtelExporter::OtlpGrpc {
        endpoint: "https://otlp.your-monitoring.com:4317".to_string(),
        headers,
    },
};
```

### 7.2 Log Aggregation Setup

**Structured Logging Configuration**:
```bash
# JSON format for machine parsing
export RUST_LOG="info,codex_core=debug,codex_app_server=debug"
export RUST_LOG_FORMAT="json"  # If supported by your logger

# Example log output
{"timestamp":"2025-12-21T10:30:45Z","level":"ERROR","target":"codex_core::client","message":"API request failed","error":"Connection timeout","request_id":"req-123"}
```

**Log Collection** (Fluent Bit example):
```yaml
# fluent-bit.conf
[INPUT]
    Name              tail
    Path              /var/log/containers/codex-app-server*.log
    Parser            docker
    Tag               kube.codex-app-server
    Refresh_Interval  5

[FILTER]
    Name                kubernetes
    Match               kube.*
    Kube_URL            https://kubernetes.default.svc:443
    Merge_Log           On
    Keep_Log            Off

[OUTPUT]
    Name  es
    Match kube.codex-app-server
    Host  elasticsearch.monitoring.svc
    Port  9200
    Index codex-logs
```

### 7.3 Dashboard Recommendations

**Key Metrics to Display**:
1. **Request Rate & Errors**
   - Total requests/sec
   - Error rate (%)
   - Status code distribution

2. **Latency Percentiles**
   - P50, P95, P99 latency
   - Request duration histogram

3. **Resource Usage**
   - CPU utilization (%)
   - Memory usage (MB)
   - Pod restart count

4. **Business Metrics**
   - Active sessions
   - API calls by endpoint
   - Token usage

**Grafana Dashboard JSON** (starter template):
```json
{
  "dashboard": {
    "title": "Codex App Server - Production",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [{"expr": "sum(rate(http_requests_total[5m]))"}]
      },
      {
        "title": "Error Rate",
        "targets": [{"expr": "sum(rate(http_requests_total{status=~\"5..\"}[5m])) / sum(rate(http_requests_total[5m]))"}]
      },
      {
        "title": "P99 Latency",
        "targets": [{"expr": "histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))"}]
      }
    ]
  }
}
```

**Prometheus Scrape Config**:
```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'codex-app-server'
    kubernetes_sd_configs:
      - role: pod
        namespaces:
          names: ['production']
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        action: keep
        regex: codex-app-server
      - source_labels: [__meta_kubernetes_pod_name]
        target_label: pod
    scrape_interval: 15s
    scrape_timeout: 10s
```
