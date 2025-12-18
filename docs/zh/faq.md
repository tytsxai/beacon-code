# 常见问题（中文）

本文聚焦中文用户常见场景，更详细的英文 FAQ 参见 `../faq.md`。

## 安装与网络

- **无法下载依赖/二进制**：尝试配置国内镜像（如 npm `registry=https://registry.npmmirror.com`，Homebrew `HOMEBREW_BREW_GIT_REMOTE` / `HOMEBREW_CORE_GIT_REMOTE` 指向镜像）。
- **代理**：启动前确保 shell 已设置 `http_proxy`/`https_proxy`/`ALL_PROXY`，并在 Codex 需要联网时保持有效。
- **npm 全局安装权限不足**：使用 `npm_config_prefix` 或 `corepack enable` + `pnpm config set prefix ~/.pnpm-global`，并将对应 bin 目录加入 PATH。

## 字体与显示

- TUI 需要支持等宽字体且覆盖中文。推荐在终端中选择包含 CJK 字形的等宽字体（如 JetBrains Mono + Nerd Font）。若出现字符对齐问题，优先更换字体或调高终端字号。
- 组合 Emoji + 中文宽度渲染异常属于终端差异，可在终端偏好里关闭字形连字/字距调整。

## 配置与语言

- 默认语言英文；建议在 `~/.code/config.toml` 设置 `ui_locale = "zh-CN"` 切换中文（缺失条目会回退英文）。
- 配置文件位置：`~/.code/config.toml`（兼容读取 `~/.codex/config.toml`）。示例见 `config-examples.md`。

## 运行环境

- 建议 Node.js 20+；如需切换版本，可用 `n` 或 `fnm`/`nvm`，并将 PATH 正确导出。
- Mac 安装 Homebrew 慢：使用国内镜像或尝试离线包。
- Linux Wayland/SSH 无法复制粘贴图片：目前需本地终端或使用文本输入；图片功能依赖支持粘贴的终端。

## 常见报错

- **权限被拒绝/只读**：检查是否在沙箱模式或权限不足目录；可用 `--sandbox workspace-write` 运行或切换到有写权限的目录。
- **模型不可用/未登录**：确认 ChatGPT 登录或 `OPENAI_API_KEY` 设置；若使用激活器，检查 `~/.codex/config.toml` 是否已同步到 `~/.code/`。
- **网络超时**：确认代理/镜像有效，必要时重试或切换网络。

## 贡献与同步

- 本地化改动集中在 `docs/zh/` 和后续的字符串资源；保持英文版不改名/不重排，便于上游合并。
- 提交 PR 前建议附上中文场景测试（中文提示词、CJK 文件名、宽字符显示）截图或说明。
