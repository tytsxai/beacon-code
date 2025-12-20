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
- [ ] **Health Check**: `/health` endpoint returns 200 OK.

## 3. Rollback Procedure
- [ ] **Trigger**: High error rate (>1%), crash loop, or critical functional regression.
- [ ] **Steps**:
    1.  Revert to the previous stable Docker image tag.
    2.  If DB migration occurred and is backward-incompatible, run the down-migration script (carefully!).
    3.  Restart services.
    4.  Verify health check.

## 4. Post-Deployment Observability
- [ ] **Logs**: Verify `app-server` logs are flowing to the aggregation system.
- [ ] **Errors**: Monitor for "Panic occurred" or unhandled exceptions in stderr.
- [ ] **Metrics**: Check CPU/Memory usage is within baseline.
