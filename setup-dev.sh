#!/bin/bash
# edge-agent macOS 开发环境初始化脚本
# 一键安装所有依赖，适合新 Mac 或新同事 onboarding

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok()   { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_err()  { echo -e "${RED}[ERR]${NC} $1"; }

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "========================================"
echo "  edge-agent 开发环境初始化"
echo "========================================"
echo ""

# 1. 检查 Homebrew
if ! command -v brew &>/dev/null; then
    log_info "安装 Homebrew..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    # M1/M2 Mac 需要额外添加到 PATH
    if [ -d "/opt/homebrew/bin" ]; then
        eval "$(/opt/homebrew/bin/brew shellenv)"
    fi
    log_ok "Homebrew 安装完成"
else
    log_ok "Homebrew 已安装"
fi

# 2. 安装 Xcode Command Line Tools
if ! xcode-select -p &>/dev/null; then
    log_info "安装 Xcode Command Line Tools..."
    xcode-select --install
    log_warn "请按弹窗提示完成安装，然后重新运行本脚本"
    exit 0
else
    log_ok "Xcode CLT 已安装"
fi

# 3. 安装 Rust
if ! command -v rustc &>/dev/null; then
    log_info "安装 Rust 工具链..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    log_ok "Rust 安装完成: $(rustc --version)"
else
    log_ok "Rust 已安装: $(rustc --version)"
fi

# 4. 安装 Node.js
if ! command -v node &>/dev/null; then
    log_info "安装 Node.js..."
    brew install node
    log_ok "Node.js 安装完成: $(node --version)"
else
    log_ok "Node.js 已安装: $(node --version)"
fi

# 5. 安装 OpenCV
if ! brew list opencv &>/dev/null; then
    log_info "安装 OpenCV（可能需要几分钟）..."
    brew install opencv
    log_ok "OpenCV 安装完成"
else
    log_ok "OpenCV 已安装: $(pkg-config --modversion opencv4 2>/dev/null || echo 'version unknown')"
fi

# 6. 配置 pkg-config 路径（OpenCV Rust 绑定需要）
SHELL_RC=""
if [ -n "$ZSH_VERSION" ] || [ "$SHELL" = "/bin/zsh" ]; then
    SHELL_RC="$HOME/.zshrc"
elif [ -n "$BASH_VERSION" ] || [ "$SHELL" = "/bin/bash" ]; then
    SHELL_RC="$HOME/.bashrc"
fi

if [ -n "$SHELL_RC" ]; then
    OPENCV_PKG="/opt/homebrew/lib/pkgconfig"
    if [ "$(uname -m)" = "x86_64" ]; then
        OPENCV_PKG="/usr/local/lib/pkgconfig"
    fi

    if ! grep -q "$OPENCV_PKG" "$SHELL_RC" 2>/dev/null; then
        log_info "配置 OpenCV pkg-config 路径到 $SHELL_RC..."
        echo "" >> "$SHELL_RC"
        echo "# OpenCV for edge-agent" >> "$SHELL_RC"
        echo "export PKG_CONFIG_PATH=\"$OPENCV_PKG:\$PKG_CONFIG_PATH\"" >> "$SHELL_RC"
        log_ok "已追加到 $SHELL_RC"
        log_warn "请执行: source $SHELL_RC 使配置生效"
    else
        log_ok "pkg-config 路径已配置"
    fi
fi

# 7. 安装前端依赖
log_info "安装前端 npm 依赖..."
cd "$PROJECT_ROOT/frontend"
npm install
log_ok "前端依赖安装完成"

# 8. 验证
echo ""
echo "========================================"
echo "  环境检查"
echo "========================================"
echo "Rust:   $(rustc --version)"
echo "Cargo:  $(cargo --version)"
echo "Node:   $(node --version)"
echo "npm:    $(npm --version)"
echo "OpenCV: $(pkg-config --modversion opencv4 2>/dev/null || echo '未检测到，请 source ~/.zshrc 后重试')"
echo ""

log_ok "开发环境初始化完成！"
echo ""
echo "下一步:"
echo "  1. 如果修改了 shell 配置，先执行: source $SHELL_RC"
echo "  2. 启动开发服务器: ./dev-start.sh"
echo "  3. 浏览器访问: http://127.0.0.1:8080"
