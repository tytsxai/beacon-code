#!/usr/bin/env bash
#
# Beacon Code 一键安装脚本
#
# 用法:
#   curl -fsSL https://raw.githubusercontent.com/tytsxai/beacon-code/main/scripts/install.sh | bash
#   或
#   ./install.sh [--version 0.6.0] [--install-dir ~/.beacon-code]
#

set -euo pipefail

# 默认配置
VERSION="${BEACON_VERSION:-latest}"
INSTALL_DIR="${BEACON_INSTALL_DIR:-$HOME/.beacon-code}"
GITHUB_REPO="tytsxai/beacon-code"
BIN_NAME="code"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# 检测操作系统和架构
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="darwin" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *)
            error "不支持的操作系统: $(uname -s)"
            exit 1
            ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64) arch="x64" ;;
        arm64|aarch64) arch="arm64" ;;
        *)
            error "不支持的架构: $(uname -m)"
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}

# 获取 Rust target triple
get_target_triple() {
    local platform="$1"
    case "$platform" in
        linux-x64)   echo "x86_64-unknown-linux-musl" ;;
        linux-arm64) echo "aarch64-unknown-linux-musl" ;;
        darwin-x64)  echo "x86_64-apple-darwin" ;;
        darwin-arm64) echo "aarch64-apple-darwin" ;;
        windows-x64) echo "x86_64-pc-windows-msvc" ;;
        *)
            error "未知平台: $platform"
            exit 1
            ;;
    esac
}

# 获取最新版本号
get_latest_version() {
    local api_url="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
    local version

    if command -v curl &>/dev/null; then
        version=$(curl -fsSL "$api_url" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
    elif command -v wget &>/dev/null; then
        version=$(wget -qO- "$api_url" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
    else
        error "需要 curl 或 wget"
        exit 1
    fi

    if [[ -z "$version" ]]; then
        error "无法获取最新版本"
        exit 1
    fi

    echo "$version"
}

# 下载文件
download() {
    local url="$1"
    local dest="$2"

    info "下载: $url"

    if command -v curl &>/dev/null; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget &>/dev/null; then
        wget -q "$url" -O "$dest"
    else
        error "需要 curl 或 wget"
        exit 1
    fi
}

# 解压 zstd 文件
extract_zst() {
    local src="$1"
    local dest="$2"

    if command -v zstd &>/dev/null; then
        zstd -d "$src" -o "$dest" --force
    elif command -v unzstd &>/dev/null; then
        unzstd "$src" -o "$dest" --force
    else
        warn "zstd 未安装，尝试下载 tar.gz 格式..."
        return 1
    fi
}

# 主安装流程
install() {
    local platform target version download_url archive_name bin_path

    platform=$(detect_platform)
    target=$(get_target_triple "$platform")

    info "检测到平台: $platform ($target)"

    # 确定版本
    if [[ "$VERSION" == "latest" ]]; then
        info "获取最新版本..."
        version=$(get_latest_version)
    else
        version="$VERSION"
    fi

    info "安装版本: v$version"

    # 创建安装目录
    mkdir -p "$INSTALL_DIR/bin"

    # 下载二进制
    local base_url="https://github.com/${GITHUB_REPO}/releases/download/v${version}"

    if [[ "$platform" == windows-* ]]; then
        archive_name="${BIN_NAME}-${target}.exe.zst"
        bin_path="$INSTALL_DIR/bin/${BIN_NAME}.exe"
    else
        archive_name="${BIN_NAME}-${target}.zst"
        bin_path="$INSTALL_DIR/bin/${BIN_NAME}"
    fi

    download_url="${base_url}/${archive_name}"

    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap "rm -rf '$tmp_dir'" EXIT

    local archive_path="$tmp_dir/$archive_name"

    download "$download_url" "$archive_path"

    # 解压
    if ! extract_zst "$archive_path" "$bin_path"; then
        # 回退到 tar.gz
        if [[ "$platform" != windows-* ]]; then
            archive_name="${BIN_NAME}-${target}.tar.gz"
            download_url="${base_url}/${archive_name}"
            archive_path="$tmp_dir/$archive_name"
            download "$download_url" "$archive_path"
            tar -xzf "$archive_path" -C "$INSTALL_DIR/bin"
        else
            error "无法解压文件，请安装 zstd"
            exit 1
        fi
    fi

    # 设置执行权限
    if [[ "$platform" != windows-* ]]; then
        chmod +x "$bin_path"
    fi

    info "安装完成: $bin_path"

    # 添加到 PATH 提示
    local shell_config=""
    case "$SHELL" in
        */bash) shell_config="$HOME/.bashrc" ;;
        */zsh)  shell_config="$HOME/.zshrc" ;;
        */fish) shell_config="$HOME/.config/fish/config.fish" ;;
    esac

    echo ""
    info "请将以下内容添加到 $shell_config:"
    echo ""
    if [[ "$SHELL" == */fish ]]; then
        echo "  set -gx PATH \"$INSTALL_DIR/bin\" \$PATH"
    else
        echo "  export PATH=\"$INSTALL_DIR/bin:\$PATH\""
    fi
    echo ""
    info "然后运行: source $shell_config"
    echo ""
    info "或直接运行: $bin_path --version"
}

# 显示帮助
show_help() {
    cat <<EOF
Beacon Code 安装脚本

用法:
    $0 [选项]

选项:
    --version <版本>      指定版本 (默认: latest)
    --install-dir <目录>  安装目录 (默认: ~/.beacon-code)
    --help               显示帮助

环境变量:
    BEACON_VERSION       同 --version
    BEACON_INSTALL_DIR   同 --install-dir

示例:
    $0                           # 安装最新版本
    $0 --version 0.6.0           # 安装指定版本
    $0 --install-dir /opt/beacon # 自定义安装目录
EOF
}

# 解析参数
while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --install-dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            exit 0
            ;;
        *)
            error "未知参数: $1"
            show_help
            exit 1
            ;;
    esac
done

# 执行安装
install
