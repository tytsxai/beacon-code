# Auto Drive 中文概要逐句校验表（v1）

本表以用户提供的“Auto Drive 中文概要（去重汇总）”为基准，逐句对照当前仓库实现与现有文档。  
结论分为：**准确** / **部分准确** / **不准确**，并给出修订要点与对应源码/文档位置。

## 逐句校验

| 概要原句 | 对应实现/文档 | 真实行为 | 结论 | 修订要点 |
|---|---|---|---|---|
| 入口与模式：TUI `/auto <目标>` 或 CLI `code exec --auto "<goal>"`；需 danger-full-access + approval=never 全自动，否则拒跑。无明确目标且历史不可推导不启动。 | `docs/auto-drive.md:6-13`，`code-rs/cli/src/main.rs:80-103` | `/auto <goal>` 与 `code exec --auto "<goal>"` 入口正确；TUI 若未在全自动环境（danger-full-access + approval=never）会警告且不启动 Auto Drive；CLI 无头模式必须给 goal；无目标且历史不可推导时不会启动。 | 部分准确 | “否则拒跑”只对 TUI 全自动要求成立；CLI Auto Drive 仍可在受限 sandbox 下运行（但审批强制 never）。 |
| 核心流程：code-auto-drive-core 的协调器用 JSON Schema 约束决策（finish_status/status/prompt_sent_to_cli 4–600 字、可选 agents），控制器管理阶段与 continue 模式（immediate/10s/60s/manual），将提示交给 CLI/TUI 执行并回填转录，历史会自动压缩。 | `code-rs/code-auto-drive-core/src/auto_coordinator.rs`，`docs/auto-drive.md:14-33` | 协调器确实用 schema 约束决策；字段包含 `finish_status`（实现中是 `finish_status`/`status` 枚举）、`status_title`、`status_sent_to_user`、`prompt_sent_to_cli`、可选 `agents`；prompt 字段 minLength=4，maxLength=600 由解析后校验强制（schema 本身故意不设 maxLength）；continue_mode 四档存在；决策 prompt 被拼接送入执行器并回填 transcript；历史会触发压缩并提示。 | 准确 | 需补一句：prompt 上限是“解析后校验”，schema 仅声明最小长度与描述。 |
| Agents 与安全：决策可含并行/阻塞代理；无 git 工作树强制只读并在提示中注明；AUTO_AGENTS.md + AGENTS.md 提供指令。 | `docs/auto-drive.md:17-22`，`code-rs/code-auto-drive-core/src/auto_coordinator.rs:1135-1143`，`code-rs/exec/src/lib.rs:823-860` | agents 支持 parallel/blocking timing；不在 git repo 时会在 developer intro 中要求降级写代理为只读；AUTO_AGENTS.md 会与 AGENTS.md 一起注入说明。 | 准确 | 无需修改。 |
| 观察与审核：默认每 `auto_drive_observer_cadence` 轮轻量审阅；可插入 review/QA/cross-check 流程，卡片会进入 Awaiting review；Esc 可随时暂停/停止。 | `docs/auto-drive.md:23-43`，`code-rs/tui/tests/snapshots/*auto_drive_review*.snap` | observer cadence 默认 5 轮；review/QA/cross-check 默认开启并会切到 Awaiting review；Esc 规则与倒计时/审批场景说明一致。 | 准确 | 无需修改。 |
| 配置与默认：配置在 `config.toml [auto_drive]`（model、use_chat_model、continue_mode、review/agents/QA/cross-check 默认开启，observer_cadence 等）；`config.toml.example` 展示推荐 gpt-5.2 + 高推理，continue_mode=ten-seconds。 | `docs/auto-drive.md:105-108`，`config.toml.example:23-47`，`code-rs/core/src/config_types.rs:773-900` | 默认值与字段位置一致；`use_chat_model` 为顶层/设置项；代码默认模型 `gpt-5.1`、reasoning `high`、continue_mode `ten-seconds`，示例配置使用 `gpt-5.2`。 | 部分准确 | 区分“默认值”与“示例/推荐值”。 |
| 增强模块（实验性）：checkpoint 持久化/恢复，diagnostics 循环/偏离/token 异常，budget 控制 token/轮次/时长，重试/退避，语义压缩，审计与遥测，高吞吐多智能体（session_pool/parallel_execution）与外部记忆/进度日志。 | `docs/auto-drive.md:57-104`，`docs/architecture.md:116-133`，`code-rs/core/src/config_types.rs:818-868` | 这些模块都存在且由 `[auto_drive]`/`[auto_drive.high_throughput]` 控制；默认 checkpoint/audit/telemetry 关闭，其余多为默认开启。 | 准确 | 可在概要中强调默认开关状态（哪些默认 off）。 |
| UI/TUI：卡片展示状态（Ready/Waiting/Thinking/Running/Awaiting review/Failed/Stopped）、目标、动作日志、倒计时、token/时间计数、庆祝动画；底部面板/设置可调模型与 continue_mode；Esc 在倒计时与审批期仍有效。 | `docs/auto-drive.md:48-51`，`code-rs/tui/tests/snapshots/*auto_drive*.snap` | UI 状态与交互描述对齐；设置可切模型与 continue_mode；Esc 透传正确。 | 准确 | 无需修改。 |
| CLI/无头：exec 路径将协调器提示拼接并执行，`--auto` 默认只读；需写/联网用 `--full-auto`，会关闭审批。 | `docs/auto-drive.md:26-29`，`code-rs/exec/src/cli.rs:13-55`，`code-rs/exec/src/lib.rs:193-226` | exec 会拼接 prompt 并执行；无头 exec 强制 `approval_policy=never`；`--auto` 不等于只读，sandbox 仍按 `--sandbox` 或配置；`--full-auto` 只是把 sandbox 设为 `workspace-write`（仍是沙箱），不是“允许联网/写且无沙箱”；`--dangerously-bypass-approvals-and-sandbox` 才是 danger-full-access。 | 不准确 | 改为：`--auto` 仅启用 Auto Drive；审批总是 never；写入能力取决于 sandbox（full-auto=workspace-write，yolo=danger-full-access）。 |
| 安全与历史：历史在内存（有 checkpoint 时落盘校验）；无目标且无法从历史推断时终止；非 git 环境写请求会降级或失败并提示。 | `docs/auto-drive.md:10-16,52-55,61-75`，`code-rs/code-auto-drive-core/src/auto_coordinator.rs:1135-1143` | 历史默认内存；checkpoint 开启会落盘并校验；无目标且不可推导即不启动/会停止；非 git 环境写代理会被降级为只读。 | 准确 | 无需修改。 |

## 修订版中文概要 v1（基于校验）

入口与模式：TUI 用 `/auto <goal>`（省略 goal 且近期历史可推导时会提议目标），CLI 用 `code exec --auto "<goal>"` 或 `code auto "<goal>"`（无头必须提供 goal）。TUI 只有在 danger-full-access + approval=never 的全自动环境下才会启动 Auto Drive；否则提示并拒绝启动。目标缺失且历史不可推导时不会启动。

核心流程：`code-auto-drive-core` 的协调器用 JSON Schema 约束每轮决策（`finish_status/status/status_title/status_sent_to_user/prompt_sent_to_cli` 与可选 `agents`）；`prompt_sent_to_cli` 最少 4 字，超过 600 字会在解析后被拒绝并触发可恢复重试（schema 本身不设 maxLength）。控制器驱动阶段与 continue_mode（immediate/ten-seconds/sixty-seconds/manual），将决策提示交给 CLI/TUI 执行，回填 transcript；历史在内存中自动压缩，裁剪时会提示。

Agents 与安全：决策可指定 agents 及并行/阻塞 timing；若当前目录不是 git repo，会在系统提示中强制所有 agents 只读（任何 `write: true` 降级为 false）。`AUTO_AGENTS.md` 与 `AGENTS.md` 会被合并注入为运行指令。

观察与审核：默认每 `auto_drive_observer_cadence` 轮做轻量审阅；review/QA/cross-check 默认开启并可插入流程，卡片可进入 Awaiting review；Esc 在运行、倒计时与审批对话中都可随时暂停/停止。

配置与默认：配置位于 `config.toml` 顶层与 `[auto_drive]`（model/use_chat_model/continue_mode/observer/review/agents/QA/cross-check 等）；代码默认 `gpt-5.1` + reasoning `high`，示例配置使用 `gpt-5.2`，continue_mode=`ten-seconds`。

增强模块（实验性）：checkpoint 持久化恢复、diagnostics 循环/偏离/token 异常、budget（token/轮次/时长）、重试退避、语义压缩、审计/遥测，以及高吞吐多智能体（session_pool/parallel_execution）与外部记忆/进度日志，均由 `[auto_drive]`/`[auto_drive.high_throughput]` 控制，部分默认关闭（checkpoint/audit/telemetry）。

UI/TUI：卡片展示状态（Ready/Waiting/Thinking/Running/Awaiting review/Failed/Stopped）、目标、动作日志、倒计时、token/时间计数与完成动画；底部设置可切模型与 continue_mode。

CLI/无头：exec 路径会拼接协调器提示执行；无头模式始终 `approval_policy=never`。写入/联网能力取决于 sandbox：`--full-auto` 设为 `workspace-write`（仍有沙箱边界），`--dangerously-bypass-approvals-and-sandbox` 才是 danger-full-access。
