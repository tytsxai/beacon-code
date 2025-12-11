## 快速上手

想直接查看？可跳转：

- [技巧与快捷键](#技巧与快捷键)——热键、恢复会话、常用提示
- [非交互模式](../exec.md)——用 `codex exec` 做自动化
- 深度自定义：参阅 [`advanced.md`](../advanced.md) 与 [`config.md`](../config.md)

### CLI 用法

| 命令              | 作用                       | 示例                               |
| ----------------- | ------------------------- | ---------------------------------- |
| `codex`           | 进入交互式 TUI            | `codex`                            |
| `codex "..."`     | 启动时带初始提示          | `codex "fix lint errors"`          |
| `codex exec "..."`| 非交互“自动化”模式        | `codex exec "explain utils.ts"`    |

常用参数：`--model/-m`（模型选择）、`--ask-for-approval/-a`（需要人工审批时提示）。

### 恢复交互会话

- `codex resume`：打开会话选择器
- 恢复最近：`codex resume --last`
- 按 id 恢复：`codex resume <SESSION_ID>`（可从 `/status` 或 `~/.codex/sessions/` 获取）
- 选择器会显示会话的工作目录和（若有）记录时的 Git 分支

示例：

```shell
codex resume
codex resume --last
codex resume 7f9f9a2e-1b3c-4c7a-9b0e-123456789abc
```

### 直接用提示启动

```shell
codex "explain this codebase to me"
```

### 示例提示

| ✨  | 输入示例                                                                    | 会发生什么                                                          |
| --- | --------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| 1   | `codex "Refactor the Dashboard component to React Hooks"`                   | 重写为 Hooks，运行 `npm test`，展示 diff。                           |
| 2   | `codex "Generate SQL migrations for adding a users table"`                  | 推断 ORM，生成迁移并在沙盒 DB 运行。                                 |
| 3   | `codex "Write unit tests for utils/date.ts"`                                | 生成测试、执行并修到通过。                                           |
| 4   | `codex "Bulk-rename *.jpeg -> *.jpg with git mv"`                           | 安全重命名并更新引用。                                               |
| 5   | `codex "Explain what this regex does: ^(?=.*[A-Z]).{8,}$"`                  | 逐步解释正则。                                                       |
| 6   | `codex "Carefully review this repo, and propose 3 high impact well-scoped PRs"` | 给出高影响、可落地的 PR 建议。                                       |
| 7   | `codex "Look for vulnerabilities and create a security review report"`      | 查找并解释安全问题。                                                 |

想复用自定义指令？可在 `docs/prompts.md` 了解自定义斜杠菜单。

### 记忆与 AGENTS.md

通过 `AGENTS.md` 给 Codex 额外指引。查找顺序：

1. `~/.codex/AGENTS.md`（全局）
2. 仓库根到当前目录路径上的 `AGENTS.override.md`（若有优先）或 `AGENTS.md`

更多用法见 [AGENTS 文档](https://agents.md/)。

### 技巧与快捷键

#### 用 `@` 搜索文件

输入 `@` 触发文件名模糊搜索，上下键选择，Tab/Enter 替换，Esc 取消。

#### Esc–Esc 编辑上一条消息

聊天框为空时按 Esc 进入回溯，再按 Esc 预览上一条用户消息；继续 Esc 可向更早消息移动，Enter 确认后会从该消息分支并预填到输入框。预览底部会显示 `Esc edit prev` 提示。

#### `--cd`/`-C` 指定工作目录

无需 `cd`，直接：

```shell
codex --cd apps/frontend --add-dir ../backend
```

会在 TUI 头部显示当前工作目录。

#### `--add-dir` 附加可写目录

多仓场景用 `--add-dir` 暴露额外可写路径，主工作目录不变。

#### Shell 补全

```shell
codex completion bash
codex completion zsh
codex completion fish
```

#### 图片输入

支持粘贴图片或用 `-i/--image`：

```bash
codex -i screenshot.png "Explain this error"
codex --image img1.png,img2.jpg "Summarize these diagrams"
```

#### 环境变量与可执行文件

在启动前准备好环境（虚拟环境、代理、必需的 bin），避免运行时额外探测耗时。
