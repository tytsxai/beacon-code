# Codex 智能体指南

本文档为 AI 智能体提供项目开发规范。

## 目录

- [项目结构](#项目结构)
- [构建与验证](#构建与验证)
- [代码规范](#代码规范)
- [提交与推送](#提交与推送)
- [架构说明](#架构说明)
- [测试指南](#测试指南)
- [发布流程](#发布流程)

---

## 项目结构

| 目录 | 说明 | 命名规范 |
|------|------|----------|
| `codex-rs/` | 上游只读镜像（`openai/codex:main`） | `codex-*` |
| `code-rs/` | 实际开发目录，所有 Rust 修改在此进行 | `code-*` |
| `codex-cli/` | TypeScript CLI 包 | - |
| `sdk/` | SDK 实现 | - |
| `shell-tool-mcp/` | MCP shell 工具 | - |

**重要**：不要直接修改 `codex-rs/`，所有改动在 `code-rs/` 下进行。

---

## 构建与验证

### 必须通过的检查

```bash
./build-fast.sh    # 唯一必跑检查，必须干净通过
```

- 冷缓存可能需要 20+ 分钟，设置足够长的超时
- 任何编译 warning 视为失败，必须修复（改名 `_`、去掉多余 `mut`、删死代码）
- **禁止运行 rustfmt**
- 未被要求时不要额外跑 `just fmt`、`just fix`、`cargo test`

### 推送前预检

```bash
./pre-release.sh   # 推到 main 前必跑
```

### 可选回归检查（改动 Rust workspace 推荐）

```bash
# 全量测试
cargo nextest run --no-fail-fast

# 快速聚焦
cargo test -p code-tui --features test-helpers
cargo test -p code-cloud-tasks --tests
cargo test -p mcp-types --tests
```

### 调试原则

先写能失败的测试或复现脚本并确认它会红，再动代码——测不红无法证明修复有效。

---

## 代码规范

### Rust

- `format!` 能直接内插变量时用 `{var}`，不用 `{}`+参数
- 优先用 `?` 传播错误，避免 `.unwrap()` 除非确定不会 panic
- 使用 `tracing` 记录日志，按重要性选择级别：`error` > `warn` > `info` > `debug` > `trace`

### TypeScript

- 遵循项目 `.prettierrc.toml` 配置
- 类型优先，避免 `any`

### 文档

- 保持简洁、清晰、最新
- 删除陈旧内容，不堆叠免责声明
- 聚焦关键流程，不记录次要功能
- 不提交临时文档、计划或草稿

---

## 提交与推送

### 提交信息规范

格式：Conventional Commit + 可选 scope

```
<type>(<scope>): <subject>

[optional body]
```

**规则**：
- 主题 ≤ 72 字符
- 祈使语气：`add`、`fix`、`update`（不用过去式）
- 提交前检查暂存区：`git --no-pager diff --staged --stat`

**示例**：
```
feat(tui/history): show exit code and duration for Exec cells
fix(core/exec): handle SIGINT to avoid orphaned child
docs(agents): clarify commit-message expectations
```

**禁止**：
- 占位符如 `chore: commit local work`
- 自造合并前缀如 `merge(main<-origin/main):`

### 推送策略（合并式，不要 rebase）

```bash
# 1. 提交本地改动
git --no-pager diff --stat
git add -A
git commit -m "<descriptive message>"

# 2. 拉取远端
git fetch origin

# 3. 合并但不自动提交
git merge --no-ff --no-commit origin/main

# 4. 解决冲突
git checkout --ours .                           # 默认保留本地
git checkout --theirs codex-cli/package.json    # 版本文件用远端

# 5. 提交合并
git add -A
git commit -m "Merge origin/main: adopt remote version bumps; keep ours elsewhere"

# 6. 验证后推送
./build-fast.sh
git push
```

**原则**：
- 不要 rebase，不要 `git pull --rebase`
- 版本元数据（如 `package.json` 版本号）默认用远端
- 非平凡冲突先停下询问

---

## 架构说明

### 命令执行（事件驱动）

```
┌─────────────────────────────────────────────────────────────┐
│  Core 层 (code-rs/core/src/codex.rs)                        │
│  - on_exec_command_begin() 发起执行                         │
│  - 创建 EventMsg::ExecCommandBegin                          │
└─────────────────────┬───────────────────────────────────────┘
                      │ 事件
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  TUI 层 (code-rs/tui/src/chatwidget.rs)                     │
│  - handle_codex_event() 处理事件                            │
│  - 管理 RunningCommand 状态                                 │
│  - 创建 HistoryCell::Exec                                   │
└─────────────────────┬───────────────────────────────────────┘
                      │ 渲染
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  History Cell (code-rs/tui/src/history_cell.rs)             │
│  - new_active_exec_command() 创建运行中 cell                │
│  - new_completed_exec_command() 完成时更新                  │
│  - ParsedCommand 处理语法高亮                               │
└─────────────────────────────────────────────────────────────┘
```

### TUI 流式内容排序

每条流式插入（Answer/Reasoning）必须带稳定键 `(request_ordinal, output_index, sequence_number)`：
- 必须带非空 stream id
- UI 基于 `OrderMeta` 播种排序键
- 无 stream id 的内容不会插入，开发期以错误日志暴露

### Auto Drive Esc 处理

| 位置 | 职责 |
|------|------|
| `chatwidget.rs::auto_should_handle_global_esc` | 决定全局 Esc 是否让位给 Auto Drive |
| `chatwidget.rs::handle_key_event` | 停止/暂停处理 |
| `auto_coordinator_view.rs` | 让 Esc 冒泡回聊天窗口（不拦截） |

**原则**：调整 Esc 语义时两处一起改；不要在其他地方新增 Esc 处理。

---

## 测试指南

### UI 回归测试

```rust
// 1. 构建 ChatWidget
let chat = make_chatwidget_manual();
// 或 make_chatwidget_manual_with_sender()

// 2. 模拟输入
chat.handle_key_event(/* ScriptStep */);
// 或用 run_script() helper

// 3. 渲染并断言
let terminal = Terminal::new(TestBackend::new(w, h));
let output = buffer_to_string(terminal.backend().buffer());
assert_snapshot!(output);
```

**要点**：
- 用 `assert_snapshot!` 或富字符串对比
- 保持确定性：修剪行尾空格，按现有测试节奏推进 commit tick
- 新增/更新快照用 `UPDATE_IDEAL=1` 控制

### VT100 快照测试

```bash
cargo test -p code-tui --test vt100_chatwidget_snapshot --features test-helpers -- --nocapture
```

- 位置：`code-rs/tui/tests/vt100_chatwidget_snapshot.rs`
- 使用 `ChatWidgetHarness` 预置历史/事件
- 单帧：`render_chat_widget_to_vt100(width, height)`
- 多帧：`render_chat_widget_frames_to_vt100(&[(w,h), ...])`
- 确定性：测试自动设 `CODEX_TUI_FAKE_HOUR=12`
- 接受快照：`cargo insta review` / `cargo insta accept`

---

## 发布流程

### 监控 GitHub Actions

```bash
# 跟踪 release workflow
scripts/wait-for-gh-run.sh --workflow Release --branch main

# 已知 run id
scripts/wait-for-gh-run.sh --run <run-id>

# 调整轮询间隔（默认 8 秒）
scripts/wait-for-gh-run.sh --interval 15

# 失败时自动 dump 日志
scripts/wait-for-gh-run.sh --failure-logs
```

**依赖**：PATH 中需要 `gh`（GitHub CLI）和 `jq`。

---

## 快速参考

| 场景 | 命令 |
|------|------|
| 构建验证 | `./build-fast.sh` |
| 推送前预检 | `./pre-release.sh` |
| 全量测试 | `cargo nextest run --no-fail-fast` |
| TUI 测试 | `cargo test -p code-tui --features test-helpers` |
| VT100 快照 | `cargo test -p code-tui --test vt100_chatwidget_snapshot --features test-helpers` |
| 监控发布 | `scripts/wait-for-gh-run.sh --workflow Release --branch main` |
