# Upstream sync policy (tuning)

This repo is a downstream fork. To reduce merge mistakes, we classify changed files into:

- **Upstream-owned**: keep close to upstream (prefer mirror-like behavior)
- **Downstream-owned**: fork-owned behavior/docs (should not be overwritten)
- **Shared**: must be manually reviewed (adopt upstream fixes while preserving fork behavior)

## Rules file

The checker reads: `scripts/upstream-merge/policy-rules.txt`

Format (tab-separated):

```
<category> <severity> <glob> <note>
```

- `category`: `upstream_owned` | `downstream_owned` | `shared`
- `severity`: `info` | `warn` | `fail`
- `glob`: bash-style pattern (e.g. `codex-rs/*`, `.github/workflows/*`)
- `note`: short guidance shown in the report

Matching behavior:

- Rules are matched **top-to-bottom**.
- The **first match** sets the category.
- Any `warn`/`fail` match is included in “High-risk matches”.

## How maintainers use it

- Generate report (recommended):
  - `just upstream-policy -- --range origin/main...HEAD`
- Generate strict report (fails CI/local command if warn/fail matches exist):
  - `just upstream-policy-strict -- --range origin/main...HEAD`

The verify script also writes the report to `.github/auto/VERIFY_policy.md`.

