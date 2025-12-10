# 上游同步指南 (Upstream Sync Guide)

本文档记录了从 `openai/codex` 上游仓库同步代码的最佳实践，确保本地增强功能不会因合并而丢失。

## 📋 同步前检查清单

### 1. 备份本地更改
```bash
# 确保所有更改已提交
git status

# 推送到 origin 备份
git push origin main

# 如有未追踪的重要文件，先 stash
git stash --include-untracked
```

### 2. 确认本地增强模块
我们的本地增强主要在以下目录，合并时需要特别保护：

| 目录/模块 | 用途 | 优先级 |
|-----------|------|--------|
| `code-auto-drive-core/` | Auto Drive 增强功能 | 🔴 最高 |
| `core/*_zh.md` | 中文提示文件 | 🟡 中等 |
| `~/.code/config.toml` | 本地配置 | 🟢 不受影响 |

## 🔄 同步流程

### Step 1: 获取上游更新
```bash
git fetch upstream
git log --oneline HEAD..upstream/main | head -20  # 查看上游新增提交
```

### Step 2: 检查冲突预览
```bash
# 使用 --no-commit 预览冲突
git merge --no-commit --no-ff upstream/main

# 如果有冲突，查看冲突文件
git diff --name-only --diff-filter=U

# 取消预览合并
git merge --abort
```

### Step 3: 执行合并（推荐 Merge 而非 Rebase）

**为什么选择 Merge：**
- 本地有大量自定义功能，Rebase 会在每个提交上产生冲突
- Merge 一次性解决所有冲突，更高效

```bash
git merge upstream/main --no-commit
```

### Step 4: 解决冲突

#### 冲突处理策略

| 冲突类型 | 处理方式 |
|----------|----------|
| **CI/CD 文件** (`.github/workflows/*`) | 接受上游版本 `git checkout --theirs <file>` |
| **文档文件** (`docs/*.md`) | 接受上游版本，中文文档单独维护 |
| **核心代码冲突** | 需要手动审查合并 |
| **本地增强模块** | 保留本地版本 `git checkout --ours <file>` |

```bash
# 批量接受上游文档
git checkout --theirs docs/*.md
git add docs/

# 保留本地增强模块
git checkout --ours code-auto-drive-core/
git add code-auto-drive-core/
```

### Step 5: 验证构建
```bash
cargo check           # 快速检查编译
cargo test --lib      # 运行测试
cargo build --release # 完整构建
```

### Step 6: 提交并推送
```bash
git commit -m "merge: sync with upstream/main (commit-hash)

Merged upstream changes including:
- [列出主要上游功能]

Preserved local enhancements:
- Auto Drive core modules
- Chinese prompt files"

git push origin main
```

## ⚠️ 常见问题

### 问题 1: 中文提示文件被覆盖
**症状**: `prompt_zh.md` 内容变成英文
**解决**: 中文提示文件需要单独维护，不要让上游覆盖

### 问题 2: 模块导入错误
**症状**: `cargo check` 报 `file not found for module`
**解决**: 检查 `lib.rs` 中的模块声明是否完整

```rust
// code-auto-drive-core/src/lib.rs 需要声明所有模块
pub mod audit;
pub mod budget;
pub mod checkpoint;
// ... 其他增强模块
```

### 问题 3: 测试失败
**症状**: 某些测试依赖特定内容的测试失败
**解决**: 检查测试断言是否依赖已被上游修改的内容，必要时更新测试

## 📁 本地增强目录结构

```
code-rs/
├── code-auto-drive-core/     # 🔴 本地增强 - 需保护
│   └── src/
│       ├── parallel_execution.rs
│       ├── checkpoint.rs
│       ├── diagnostics.rs
│       ├── budget.rs
│       ├── scheduler.rs
│       ├── telemetry.rs
│       └── ... (其他增强模块)
├── core/
│   ├── prompt.md             # 上游
│   ├── prompt_zh.md          # 🟡 本地中文 - 需单独维护
│   └── *_zh.md               # 🟡 其他中文提示
└── docs/
    └── UPSTREAM_SYNC_GUIDE.md  # 本文档
```

## 📊 合并统计模板

每次合并后建议记录：

```
日期: YYYY-MM-DD
上游 commit: <hash>
冲突文件数: N
解决方式: [接受上游/保留本地/手动合并]
构建状态: ✅/❌
测试状态: ✅/❌ (passed/failed)
```

---

*最后更新: 2025-12-10*
