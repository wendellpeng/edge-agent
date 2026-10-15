#!/bin/bash
# edge-agent macOS 服务卸载脚本

PLIST_FILE="$HOME/Library/LaunchAgents/com.edgeagent.app.plist"

if [ -f "$PLIST_FILE" ]; then
    launchctl unload "$PLIST_FILE" 2>/dev/null || true
    rm -f "$PLIST_FILE"
    echo "✅ edge-agent 开机自启服务已卸载"
else
    echo "服务未安装"
fi
