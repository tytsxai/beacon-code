# 上游同步指南

本项目是 [openai/codex](https://github.com/openai/codex) 的社区分支（downstream）。为保持功能完整性、安全性与可维护性，我们需要以**固定流程**、**小步频繁**地同步上游更新。

> [!NOTE]
> 本仓库包含两套 Rust workspace：`code-rs/`（Every Code 主开发）与 `codex-rs/`（上游镜像/对照用）。
>
> - **目标**：将“我们的特色”尽量集中在 `code-rs/`；将 `codex-rs/` 视为“上游参考镜像”，避免在其中做长期漂移的产品化改动。
> - **原因**：上游更新很频繁，若在上游频繁变动的核心文件上做大量自定义，会显著放大冲突与回归风险。

## 结论（最佳方案）

> [!IMPORTANT]
> **推荐的“最佳方案”是：下游主线使用 `merge` 的 Upstream Merge Train 模式**（不要对 `main` 做长期 `rebase`）。
>
> - `merge upstream/main` 一次性解决冲突、保留真实合并点，适合本仓库这种“中等以上差异”的 downstream。
> - 通过小步频繁同步 + 冲突复用（`rerere`）+ 自动化验证（脚本），把同步从“灾难性大工程”降级为“常规维护”。

## 同步原则（降低干扰与错误率）

1. **只通过 PR 合并上游**：不要直接在 `main` 上 merge 后 push（减少误操作与回滚难度）。
2. **固定分支命名**：同步分支统一用 `sync/upstream-YYYYMMDD`，避免在 `main` 上直接操作。
3. **保持 `codex-rs/` 可对照**：冲突时优先让 `codex-rs/` 更贴近上游；把产品化改动尽量放到 `code-rs/`。
4. **冲突策略可复用**：启用 `git rerere`，让同类冲突下次自动套用解决方式。
5. **合并后必做验证**：至少跑一次 `./build-fast.sh` + 关键 crate 的测试；UI 变更要关注快照/渲染差异。
6. **记录合并决策**：每次上游同步都要留下“上游引用 + 决策 + 风险点”，方便以后复盘与快速解决重复冲突。

## 一次性准备（建议所有维护者都做）

### 1) 配置 upstream remote（首次）

```bash
git remote add upstream https://github.com/openai/codex.git
git fetch upstream --prune
```

### 2) 打开冲突复用（强烈建议）

```bash
git config rerere.enabled true
```

## 注意：本仓库已禁用 GitHub Actions

上游同步请使用本地手动流程或外部 CI；以下流程默认不依赖 GitHub Actions。

## 推荐路径：本地手动同步

为了降低误操作，本仓库提供了一个“只做安全准备”的辅助脚本：

```bash
./scripts/upstream-sync.sh --log
# 或（使用 justfile，便于记忆）
just upstream-sync -- --log
```

它会创建同步分支并（默认）预演合并以列出冲突文件；只有在你显式传入 `--merge` 时才会真正执行 merge。

### 0) 前置检查（避免误操作）

```bash
git status -sb
git fetch origin --prune
git fetch upstream --prune
```

### 1) 建同步分支（不要在 main 上直接操作）

```bash
git checkout main
git pull --ff-only origin main

branch="sync/upstream-$(date +%Y%m%d)"
git checkout -b "$branch"
```

### 2) 预演冲突（可选但很推荐）

```bash
git merge --no-commit --no-ff upstream/main || true
git diff --name-only --diff-filter=U
git merge --abort
```

### 3) 正式合并上游（merge train）

```bash
git merge --no-ff upstream/main
```

### 4) 冲突处理策略（统一口径）

把冲突分成三类来处理，能显著降低错误率：

1) **Upstream-owned（跟上游走）**
- `codex-rs/**`：优先贴近上游，避免把产品化改动长期放在镜像里

2) **Downstream-owned（我们接管）**
- `docs/zh/**`：中文本地化文档
- `code-rs/code-auto-drive-core/**`：fork 专有增强模块

3) **Shared（必须人工审查）**
- `code-rs/**`：我们的主开发区；按“采纳上游改进 + 保留我们的行为差异”逐块审查
- `README.md` / `docs/*.md`：需要把上游新增内容合进来，同时保留我们的特色章节

> 实操建议：如果你不确定某个文件属于哪一类，默认把它当作 **Shared**，先理解上游变更意图再合并。

### 5) 统一验证（减少回归）

至少执行以下之一：

```bash
./build-fast.sh
./scripts/upstream-merge/verify.sh
# or
just upstream-verify
```

验证脚本会额外生成一份“变更分类（policy）报告”：

- 输出文件：`.github/auto/VERIFY_policy.md`
- 也可以手动运行：`./scripts/upstream-merge/policy-check.sh --range origin/main...HEAD` 或 `just upstream-policy`
- 需要更严格时可用：`just upstream-policy-strict -- --range origin/main...HEAD`
- 规则可在 `scripts/upstream-merge/policy-rules.txt` 调整，说明见 `docs/maintenance/upstream-sync-policy.md`

如果你在 `code-rs/` 改了具体 crate，补跑：

```bash
cd code-rs
cargo test -p <crate-you-touched>
```

### 6) 记录合并日志（强制降低“知识丢失”）

仓库自带日志工具（推荐每次同步都使用）：

```bash
./scripts/upstream-merge/log-merge.sh init upstream/main
./scripts/upstream-merge/log-merge.sh note conflict "..."
./scripts/upstream-merge/log-merge.sh decision <module> <action> "<reason>"
./scripts/upstream-merge/log-merge.sh finalize
```

它会写入：`docs/maintenance/upstream-merge-logs/`。目录说明见：`docs/maintenance/upstream-merge-logs/README.md`。

### 7) 推送并开 PR

```bash
git push -u origin "$branch"
```

PR 标题建议统一：

- `merge: sync with upstream/main (<upstream-commit>)`

PR 描述建议包含统一摘要（便于 reviewer 快速扫风险点）：

```bash
./scripts/upstream-merge/pr-summary.sh
# 或
just upstream-pr-summary
```

## 常见错误与规避

- **错误：在 `main` 上直接 merge 并 push** → 统一走同步分支 + PR。
- **错误：对 `main` 做长期 rebase** → 本仓库是 downstream，长期 rebase 会让冲突与回滚成本变高。
- **错误：把产品化改动散落在 `codex-rs/`** → 尽量迁移到 `code-rs/`，保持 `codex-rs/` 可对照。
- **错误：同步不做记录** → 下次遇到同类冲突会重复踩坑；用 `log-merge.sh` 把决策沉淀下来。

## 相关资源

- 验证脚本：`scripts/upstream-merge/verify.sh`
- 差异分析：`scripts/upstream-merge/diff-crates.sh`、`scripts/upstream-merge/highlight-critical-changes.sh`
- PR 模板：`.github/PULL_REQUEST_TEMPLATE/upstream-merge.md`
- 上游仓库：https://github.com/openai/codex
