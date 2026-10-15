#!/bin/bash
# edge-agent macOS 开机自启服务安装脚本
# 用法: ./install-macos-service.sh [--release]
#   --release  使用 target/release/edge-agent（需先 cargo build --release）

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
USE_RELEASE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --release) USE_RELEASE=true; shift ;;
        *) echo "未知参数: $1"; exit 1 ;;
    esac
done

PLIST_DIR="$HOME/Library/LaunchAgents"
PLIST_FILE="$PLIST_DIR/com.edgeagent.app.plist"
LOG_DIR="$HOME/Library/Logs/edge-agent"

mkdir -p "$PLIST_DIR"
mkdir -p "$LOG_DIR"

if [ "$USE_RELEASE" = true ]; then
    BINARY="$PROJECT_ROOT/target/release/edge-agent"
    if [ ! -f "$BINARY" ]; then
        echo "❌ release 二进制不存在"
        echo "   先执行: cd $PROJECT_ROOT && cargo build --release"
        exit 1
    fi
fi

cat > "$PLIST_FILE" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.edgeagent.app</string>
EOF

if [ "$USE_RELEASE" = true ]; then
    cat >> "$PLIST_FILE" <<EOF
    <key>ProgramArguments</key>
    <array>
        <string>$BINARY</string>
    </array>
EOF
else
    cat >> "$PLIST_FILE" <<EOF
    <key>ProgramArguments</key>
    <array>
        <string>cargo</string>
        <string>run</string>
    </array>
    <key>WorkingDirectory</key>
    <string>$PROJECT_ROOT</string>
EOF
fi

cat >> "$PLIST_FILE" <<EOF
    <key>EnvironmentVariables</key>
    <dict>
        <key>BIND_ADDR</key>
        <string>0.0.0.0:8080</string>
        <key>RUST_LOG</key>
        <string>info</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$LOG_DIR/out.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/err.log</string>
</dict>
</plist>
EOF

# 卸载旧服务（如果存在）
launchctl unload "$PLIST_FILE" 2>/dev/null || true

# 加载新服务
launchctl load "$PLIST_FILE"

echo "✅ edge-agent 已注册为 macOS 开机自启服务"
echo ""
echo "服务信息:"
echo "  plist 文件: $PLIST_FILE"
echo "  日志目录:   $LOG_DIR"
echo "  管理页面:   http://0.0.0.0:8080"
echo ""
echo "常用命令:"
echo "  查看状态:   launchctl list | grep com.edgeagent"
echo "  查看日志:   tail -f $LOG_DIR/err.log"
echo "  手动停止:   launchctl unload $PLIST_FILE"
echo "  手动启动:   launchctl load $PLIST_FILE"
