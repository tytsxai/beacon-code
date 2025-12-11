# 安装与环境（中文）

> 英文版参见 `../install.md`。本页聚焦中文用户常见网络/镜像/权限问题，保持与上游兼容，仅新增说明。

## 前置
- 推荐 Node.js 20+。如需切换版本，可使用 `n` 或 `fnm`/`nvm`，并把对应 bin 加入 PATH。
- 保证终端字体包含 CJK（等宽），避免对齐问题。

## 快速安装（默认路径）

```bash
npm install -g @just-every/code
code  # 若与 VS Code 冲突，使用 coder
```

如果 npm 权限受限，可先设置全局前缀：

```bash
export npm_config_prefix="${npm_config_prefix:-$HOME/.npm-global}"
mkdir -p "$npm_config_prefix/bin"
export PATH="$npm_config_prefix/bin:$PATH"
```

## 国内网络/镜像建议

- npm 设置镜像：`npm config set registry https://registry.npmmirror.com`
- 需要代理时，提前在 shell 设置：

```bash
export http_proxy=http://127.0.0.1:7890
export https_proxy=$http_proxy
export ALL_PROXY=$http_proxy
```

- Homebrew（macOS）慢：可将 `HOMEBREW_BREW_GIT_REMOTE`、`HOMEBREW_CORE_GIT_REMOTE` 指向镜像，或使用离线包。

## 运行与验证

```bash
code --version
code --help
```

非交互模式示例：

```bash
code exec "echo hello"
```

## ChatGPT/API Key 登录

- ChatGPT：运行 `code` 选择 “Sign in with ChatGPT”。
- API Key：`export OPENAI_API_KEY=xxx` 后运行 `code`。

若使用激活器（`codex-activator`），保持 `~/.codex/config.toml` 与 `~/.code/config.toml` 同步。

## 常见问题

- **权限被拒绝**：确认安装路径可写；或用 `npm_config_prefix` 方案。
- **网络超时**：检查代理/镜像，必要时重试或切换网络。
- **字体/对齐异常**：选择包含中文的等宽字体（如 JetBrains Mono Nerd Font），并在终端关闭字形连字。

更多排查见 `faq.md`。
