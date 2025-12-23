# Beacon Code

&ensp;

**Beacon Code**（简称 Code）是一款运行在终端里的本地快速编程智能体，专注真实的开发体验：浏览器集成、多智能体、主题与推理控制。

> 📚 中文文档入口：[docs/zh/index.md](docs/zh/index.md)  
> 💻 推荐使用 `code-rs/` 作为主要代码目录（Rust 主实现）；`beacon-cli/` 用于 CLI 包装与发布；`third_party/upstream/codex-rs/` 是上游对照快照（只读参考）。

&ensp;

## v0.5.0 有哪些更新（2025 年 11 月 21 日）

- **更名为 Beacon Code**——便于被发现，仍保留 `code` 这个简写。
- **Auto Drive 升级**——给 `/auto` 一个任务，它会自行规划、协调智能体、重跑检查并在异常时恢复，无需人工看护。
- **大量易用性改进**——`/resume` 和 `/undo` 可靠运行，补齐 compaction v2 与 -max/-mini 模型等关键能力。
- **统一设置中心**——`/settings` 集中管理限额、模型路由、主题和 CLI 集成，一处即可审计配置。
- **卡片式活动视图**——智能体、浏览器会话、网络搜索和 Auto Drive 以卡片呈现，可展开查看完整日志。
- **性能加速**——历史渲染与流式展示经过优化，即便长时间多智能体会话也保持流畅。
- **更聪明的智能体**——`/plan`、`/code`、`/solve` 使用内置子智能体并行完成任务。

完整变更见 `docs/release-notes/RELEASE_NOTES.md`。

&ensp;

## 为什么选择 Beacon Code

- 🚀 **Auto Drive 编排**——多智能体自动化，能自愈并交付完整任务。
- 🌐 **浏览器集成**——CDP 支持、无头浏览、截图内嵌。
- 🤖 **多智能体命令**——`/plan`、`/code`、`/solve` 协同多个内置子智能体。
- 🧭 **统一设置中心**——`/settings` 覆盖限额、主题、审批与提供商接入。
- 🎨 **主题系统**——可切换无障碍主题、定制强调色、通过 `/themes` 即时预览。
- 🔌 **MCP 支持**——可扩展文件系统、数据库、API 或自定义工具。
- 🔒 **安全模式**——只读、审批与工作区沙箱。

&ensp;

## 快速开始

### 直接运行

```bash
npx -y @tytsxai/beacon-code
```

### 安装并运行

```bash
npm install -g @tytsxai/beacon-code
beacon // 主命令
code   // 兼容短命令；若被 VS Code 占用可用 `coder`
```

注意：`beacon` 总是可用；若已有 `code` 命令（如 VS Code），CLI 也会安装 `coder` 作为兼容入口。

**认证方式**（三选一）：

- **ChatGPT 登录**（Plus/Pro/Team；使用你计划可用的模型）
  - 运行 `code` 选择 "Sign in with ChatGPT"
- **API Key**（按量计费）
  - 设置 `export OPENAI_API_KEY=xyz` 然后运行 `code`
    Beacon Code 默认只使用内置模型与能力，不需要额外安装其他 AI CLI。

&ensp;

## 命令

### 浏览器

```bash
# 连接外部 Chrome（CDP）
/chrome        # 自动检测端口连接
/chrome 9222   # 指定端口连接

# 切换到内置浏览器模式
/browser       # 使用内置无头浏览器
/browser https://example.com  # 在内置浏览器中打开 URL
```

### Agents

```bash
# 规划改动（多智能体共识）
/plan "Stop the AI from ordering pizza at 3AM"

# 解决复杂问题（多智能体竞速）
/solve "Why does deleting one user drop the whole database?"

# 写代码！（多智能体共识）
/code "Show dark mode when I feel cranky"
```

### Auto Drive

```bash
# 交给多步骤自动化；Auto Drive 会协调智能体和审批
/auto "Refactor the auth flow and add device login"

# 恢复或查看进行中的 Auto Drive
/auto status
```

### 通用

```bash
# 试用新主题
/themes

# 调整推理力度
/reasoning low|medium|high

# 切换模型或努力档
/model

# 开启新对话
/new
```

## CLI 参考

```shell
code [options] [prompt]

Options:
  --model <name>        覆盖模型（例如 gpt-5.1-code）
  --read-only          阻止文件修改
  --no-approval        跳过审批提示（谨慎使用）
  --config <key=val>   覆盖配置项
  --oss                使用本地开源模型
  --sandbox <mode>     设置沙箱级别（read-only、workspace-write 等）
  --help              显示帮助
  --debug             将 API 请求/响应写入日志
  --version           显示版本号
```

&ensp;

## 记忆与项目文档

Beacon Code 可在会话间记忆上下文：

1. **在项目根创建 `AGENTS.md`**：

```markdown
# Project Context

This is a React TypeScript application with:

- Authentication via JWT
- PostgreSQL database
- Express.js backend

## Key files:

- `/src/auth/` - Authentication logic
- `/src/api/` - API client code
- `/server/` - Backend services
```

2. **会话记忆**：保留对话历史
3. **代码库分析**：自动理解项目结构

&ensp;

## 非交互 / CI 模式

适用于自动化与 CI/CD：

```shell
# 运行特定任务
code --no-approval "run tests and fix any failures"

# 生成报告
code --read-only "analyze code quality and generate report"

# 批处理
code --config output_format=json "list all TODO comments"
```

## 从源码构建（开发者）

- 安装依赖：`./scripts/bootstrap.sh`
- 构建：`make build-dev`（或 `./build-fast.sh`）
- 运行：`just tui -- "explain this codebase to me"`

&ensp;

## Model Context Protocol (MCP)

Beacon Code 支持 MCP 扩展能力：

- **文件操作**：高级文件系统访问
- **数据库连接**：查询与修改数据库
- **API 集成**：连接外部服务
- **自定义工具**：构建自定义扩展

在 `~/.code/config.toml` 配置 MCP。为每个服务创建命名表，例如 `[mcp_servers.<name>]`（与其他客户端使用的 `mcpServers` JSON 对象对应）：

```toml
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/project"]
```

&ensp;

## 配置

主配置文件：`~/.code/config.toml`

> [!NOTE]
> 配置默认写入 `~/.code/`，如需自定义可设置 `CODE_HOME`。

```toml
# Model settings
model = "gpt-5.2"
model_provider = "openai"

# Behavior
approval_policy = "on-request"  # untrusted | on-failure | on-request | never
model_reasoning_effort = "medium" # low | medium | high
sandbox_mode = "workspace-write"

# UI preferences see THEME_CONFIG.md
[tui.theme]
name = "light-photon"

# Add config for specific models
[profiles.gpt-5]
model = "gpt-5.2"
model_provider = "openai"
approval_policy = "never"
model_reasoning_effort = "high"
model_reasoning_summary = "detailed"
```

### 环境变量

- `CODE_HOME`：自定义配置目录位置
- `OPENAI_API_KEY`：使用 API Key 而非 ChatGPT 登录
- `OPENAI_BASE_URL`：使用备用 API 端点
- `OPENAI_WIRE_API`：强制内置 OpenAI 提供商使用 `chat` 或 `responses` 接口

&ensp;

## FAQ

**有哪些特色？**

> Beacon Code 提供浏览器集成、多智能体命令（`/plan`、`/solve`、`/code`）、主题系统与推理控制等能力。

**可以复用现有配置吗？**

> 可以。将既有配置放到 `~/.code/` 即可继续使用；如需隔离环境，可改用 `CODE_HOME` 指向新的目录。

**能配合 ChatGPT Plus 吗？**

> 完全可以。沿用原有的 “Sign in with ChatGPT” 流程。

**数据安全吗？**

> 安全。认证留在本机，我们不会代理你的凭据或对话。

&ensp;

## 贡献

欢迎贡献！Beacon Code 以清晰的开发体验和可靠的终端工作流为目标，欢迎提交问题与改进。

### 开发流程

```bash
# 克隆与安装依赖
git clone https://github.com/tytsxai/beacon-code.git
cd code
npm install

# 构建（开发时使用快速构建）
./build-fast.sh

# 本地运行
./code-rs/target/dev-fast/code
```

### 提交 Pull Request

1. Fork 仓库
2. 创建特性分支：`git checkout -b feature/amazing-feature`
3. 实施改动
4. 运行测试：`cargo test`
5. 确认构建通过：`./build-fast.sh`
6. 提交 PR

&ensp;

## 法律与使用

### 许可证与归属

- 本项目采用 **Apache-2.0** 许可证，详情见 [LICENSE](LICENSE)。
- **Beacon Code**（Code）**并非**任何模型提供商的关联或认可项目。

### 你的责任

通过 Beacon Code 使用 OpenAI、Anthropic 或 Google 服务即表示你同意**它们的条款与政策**。尤其：

- **不要** 在非预期路径下抓取/提取内容。
- **不要** 绕过或干扰限流、配额或安全措施。
- 使用你**自己的**账号；不要共享或轮换账号以逃避限制。
- 若配置其他模型提供商，你需遵守相应条款。

### 隐私

- 认证文件位于 `~/.code/auth.json`
- 你发送给模型的输入/输出遵循各提供商条款与隐私政策；请查看这些文档（以及组织级数据共享设置）。

### 可能变更

AI 提供商可能调整资格、限额、模型或认证流程。Beacon Code 同时支持 ChatGPT 登录与 API Key 模式，可按需选择（本地/爱好 vs CI/自动化）。

&ensp;

## 许可证

Apache 2.0 - 详见 [LICENSE](LICENSE)。

Beacon Code 是独立项目，聚焦终端开发体验与可控自动化。

## &ensp;

**需要帮助？** 在 [GitHub](https://github.com/tytsxai/beacon-code/issues) 提交 issue 或查看我们的文档。
