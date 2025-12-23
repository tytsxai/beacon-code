# 配置示例（中文）

> 主配置位于 `~/.code/config.toml`。下列示例以中文用户常见需求为导向，会显式设置模型等配置，可能覆盖默认值。

## 基础示例：指定模型与审批策略

```toml
model = "gpt-5.2"
model_provider = "openai"
approval_policy = "on-request"  # 需要时询问审批
sandbox_mode = "workspace-write"  # 允许修改当前工作区
```

## 使用代理/镜像

在 shell 中设置（示例）：

```bash
export http_proxy=http://127.0.0.1:7890
export https_proxy=$http_proxy
```

若 npm 访问缓慢，可在 `~/.npmrc` 添加：

```
registry=https://registry.npmmirror.com
```

## 区分不同模型配置（Profiles）

```toml
[profiles.gpt5]
model = "gpt-5.2"
model_provider = "openai"
approval_policy = "never"
model_reasoning_effort = "high"
```

启动时通过 `--profile gpt5` 使用。

## MCP 服务器示例

```toml
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/project"]
```

## 通知脚本（macOS 例）

```toml
[notify]
command = "terminal-notifier"
args = ["-title", "Beacon", "-message", "{message}"]
```

## 语言与本地化

当前默认英文；建议通过配置项切换：

```toml
# ~/.code/config.toml
ui_locale = "zh-CN"
```

说明：

- `ui_locale` 使用 BCP 47 标签（例如 `zh-CN`、`en-US`）。
- 若中文条目缺失，会自动回退英文以避免功能回归。
