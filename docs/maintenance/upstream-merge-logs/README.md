# Upstream merge logs

This directory stores upstream merge logs created by `./scripts/upstream-merge/log-merge.sh`.
The logs capture conflict notes and decisions so future syncs can reuse the context.

## Workflow

1. Initialize a log: `./scripts/upstream-merge/log-merge.sh init upstream/main`
2. Add notes during conflict resolution:
   - `./scripts/upstream-merge/log-merge.sh note conflict "<summary>"`
   - `./scripts/upstream-merge/log-merge.sh decision <module> <action> "<reason>"`
3. Finalize once the merge is complete: `./scripts/upstream-merge/log-merge.sh finalize`

## Conventions

- One log per upstream sync PR.
- File naming: `merge-YYYYMMDD.md`.
- Keep notes short and actionable (what changed, why, and where to recheck).
