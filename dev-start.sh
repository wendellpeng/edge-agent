#!/bin/bash
# 视觉巡检智能体 macOS 开发机本地一键启动脚本
# 用法: ./dev-start.sh [options]
#   --skip-build    跳过前端构建（dist 已存在时）
#   --backend-only  只启动后端（不重建前端）
#   --frontend-dev  同时启动前端 dev server（热重载，端口 5173）
#   --clean         清理构建产物后启动

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_DIR="$PROJECT_ROOT/frontend"
BACKEND_DIR="$PROJECT_ROOT"
BIND_ADDR="${BIND_ADDR:-127.0.0.1:8080}"

SKIP_BUILD=false
BACKEND_ONLY=false
FRONTEND_DEV=false
CLEAN=false

# 解析参数
while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-build) SKIP_BUILD=true; shift ;;
        --backend-only) BACKEND_ONLY=true; shift ;;
        --frontend-dev) FRONTEND_DEV=true; shift ;;
        --clean) CLEAN=true; shift ;;
        -h|--help)
            echo "用法: ./dev-start.sh [options]"
            echo ""
            echo "选项:"
            echo "  --skip-build     跳过前端构建（dist 已存在时）"
            echo "  --backend-only   只启动后端（不重建前端）"
            echo "  --frontend-dev   同时启动前端 dev server（热重载，端口 5173）"
            echo "  --clean          清理构建产物后启动"
            echo "  -h, --help       显示帮助"
            echo ""
            echo "环境变量:"
            echo "  BIND_ADDR        绑定地址（默认: 127.0.0.1:8080）"
            echo "  RUST_LOG         日志级别（默认: info）"
            exit 0
            ;;
        *) echo "未知参数: $1"; exit 1 ;;
    esac
done

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok()   { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_err()  { echo -e "${RED}[ERR]${NC} $1"; }

# ===== 清理 =====
if [ "$CLEAN" = true ]; then
    log_info "清理构建产物..."
    rm -rf "$FRONTEND_DIR/dist"
    rm -rf "$FRONTEND_DIR/node_modules"
    rm -rf "$BACKEND_DIR/target"
    log_ok "清理完成"
fi

# ===== 检查依赖 =====
log_info "检查开发环境..."

# Rust
cargo_path=$(which cargo 2>/dev/null || true)
rustc_path=$(which rustc 2>/dev/null || true)
if [ -z "$cargo_path" ] || [ -z "$rustc_path" ]; then
    log_err "Rust 工具链未安装"
    echo "  安装命令: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "  装完后执行: source \"\$HOME/.cargo/env\""
    exit 1
fi
log_ok "Rust: $(rustc --version) / $(cargo --version)"

# Node.js
node_path=$(which node 2>/dev/null || true)
npm_path=$(which npm 2>/dev/null || true)
if [ -z "$node_path" ] || [ -z "$npm_path" ]; then
    log_err "Node.js 未安装"
    echo "  建议通过 Homebrew 安装: brew install node"
    exit 1
fi
node_version=$(node --version | sed 's/v//')
required_node="18.0.0"
if [ "$(printf '%s\n' "$required_node" "$node_version" | sort -V | head -n1)" != "$required_node" ]; then
    log_err "Node.js 版本过低: $node_version（需要 ≥ $required_node）"
    exit 1
fi
log_ok "Node.js: $(node --version) / npm $(npm --version)"

# OpenCV (可选，当前版本已不再需要)
if [ "$(uname -s)" = "Darwin" ]; then
    if ! pkg-config --exists opencv4 2>/dev/null; then
        log_warn "OpenCV 4 未通过 pkg-config 检测到（当前版本不再需要，可忽略）"
    else
        opencv_version=$(pkg-config --modversion opencv4)
        log_ok "OpenCV: $opencv_version"
    fi
fi

# ===== 前端构建 =====
if [ "$BACKEND_ONLY" = false ] && [ "$SKIP_BUILD" = false ]; then
    log_info "构建前端..."
    cd "$FRONTEND_DIR"

    if [ ! -d "node_modules" ]; then
        log_info "安装 npm 依赖..."
        npm install
    fi

    npm run build
    log_ok "前端构建完成 -> frontend/dist/"
    cd "$PROJECT_ROOT"
fi

# ===== 启动 =====
log_info "启动 视觉巡检智能体..."
log_info "绑定地址: $BIND_ADDR"
log_info "管理页面: http://$BIND_ADDR"

cd "$BACKEND_DIR"

# 设置环境变量
export BIND_ADDR
export RUST_LOG="${RUST_LOG:-info}"

# 确保缓存目录存在（macOS 下 snapshots 保存路径）
mkdir -p "$HOME/Library/Caches/com.edgeagent.app/snapshots"

# 如果带了 --frontend-dev，后台启动前端 dev server
FRONTEND_PID=""
if [ "$FRONTEND_DEV" = true ]; then
    log_info "同时启动前端 dev server (http://localhost:5173)..."
    cd "$FRONTEND_DIR"
    if [ ! -d "node_modules" ]; then
        npm install
    fi
    npm run dev &
    FRONTEND_PID=$!
    cd "$PROJECT_ROOT"
    log_ok "前端 dev server PID: $FRONTEND_PID"
fi

# 捕获退出信号，优雅关闭
cleanup() {
    echo ""
    log_info "收到退出信号，正在关闭..."
    if [ -n "$FRONTEND_PID" ]; then
        kill "$FRONTEND_PID" 2>/dev/null || true
        log_ok "前端 dev server 已停止"
    fi
    # cargo run 会在当前进程，直接退出即可
    log_ok "视觉巡检智能体 已停止"
    exit 0
}
trap cleanup INT TERM

# 启动后端（cargo run 会在前台，直到 Ctrl+C）
log_ok "视觉巡检智能体 已启动，按 Ctrl+C 停止"
echo "────────────────────────────────────────────"
cargo run
