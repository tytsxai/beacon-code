# 高吞吐多智能体配置与外部记忆

- **池默认值**：`max_sessions=20`，`min_sessions=5`，`scale_up_threshold=0.8`，`scale_down_threshold=0.3`，`backpressure_multiplier=10`（队列阈值=`max_sessions * multiplier`），`max_concurrent_agents=8`。
- **配置入口**：`[auto_drive.high_throughput]` 控制池参数；`max_concurrent_agents` 直接位于 `[auto_drive]`。
- **外部记忆**：
  - `ai/feature_list.json`（foreman 兼容）：包含 `id/description/module/priority/status/acceptance/test_requirements/tags/version/tdd_mode/verification`。
  - `ai/progress.log`：追加行格式 `timestamp | type | status | tests | summary | note`，用于 STEP/CHANGE/VERIFY/REPLAN 记录。
- **选择性测试**：根据 `git diff` 触发 Backlog 特性匹配；严格 TDD (`tdd_mode=strict`) 缺少 `test_requirements.unit` 时标记为缺测。
- **事件与告警**：`SessionSlow/SessionStuck/SessionMigrated` 诊断告警，`BackpressureWarning/Exceeded` 预算告警；并发低于 8 会生成 `LowConcurrency` 告警。

> 目标：支撑“20 并发会话 × 8 角色”吞吐，并在背压、卡死、迁移等场景下输出可观测事件。***
