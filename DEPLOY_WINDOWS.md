# 视觉巡检智能体 · Windows 部署打包指南

> 适用平台：Windows 10 / Windows 11（x86_64）
> 输出物：单文件 `edge-agent.exe` 或 `.msi` 安装包

---

## 前置条件

在 **Windows 机器**（物理机/虚拟机均可）上安装以下工具：

| 工具 | 用途 | 下载/安装命令 |
|---|---|---|
| **Rust** | 编译后端 | https://rustup.rs/ |
| **Node.js ≥ 18** | 构建前端 | https://nodejs.org/ |
| **WiX Toolset v3** | 生成 MSI 安装包 | https://github.com/wixtoolset/wix3/releases |
| **cargo-wix** | WiX 的 cargo 插件 | `cargo install cargo-wix` |

> ⚠️ **WiX Toolset 是 Windows only**，macOS/Linux 上无法运行，必须在 Windows 上执行打包。

---

## 方案一：ZIP 绿色包（最快，推荐内测）

不需要 WiX，编译完直接一个 exe 就能跑。

### 1. 构建前端

```powershell
cd edge-agent\frontend
npm install
npm run build
cd ..
```

### 2. 编译 Windows 二进制

```powershell
# 已在 .cargo/config.toml 中配置静态链接 CRT，无需额外 VC++ 运行库
cargo build --release --target x86_64-pc-windows-msvc
```

产物：`target\x86_64-pc-windows-msvc\release\edge-agent.exe`

### 3. 打包 ZIP

```powershell
# 创建绿色包目录
mkdir edge-agent-windows
Copy-Item target\x86_64-pc-windows-msvc\release\edge-agent.exe edge-agent-windows\

# 打压缩包
Compress-Archive -Path edge-agent-windows\* -DestinationPath edge-agent-v0.2.0-windows.zip
```

**使用方式**：解压 → 双击 `edge-agent.exe` → 浏览器访问 `http://localhost:8080`

---

## 方案二：MSI 安装包（正式交付，推荐）

### 1. 前置：安装 WiX Toolset

下载并安装 WiX v3.14.x：
https://github.com/wixtoolset/wix3/releases

安装后确保 `candle.exe` 和 `light.exe` 在 PATH 中。

### 2. 初始化 WiX 配置（只需一次）

```powershell
cargo wix init
```

会在 `wix\main.wxs` 生成默认安装配置。可自定义：
- 安装路径（默认 `C:\Program Files\视觉巡检智能体`）
- 是否添加 PATH
- 是否创建桌面快捷方式

### 3. 构建 MSI

```powershell
# 自动编译 release + 生成 MSI
cargo wix --target x86_64-pc-windows-msvc
```

产物：`target\wix\edge-agent-0.2.0-x86_64.msi`

### 4. 静默安装（可选）

```powershell
# 静默安装到默认路径
msiexec /i edge-agent-0.2.0-x86_64.msi /qn

# 静默卸载
msiexec /x edge-agent-0.2.0-x86_64.msi /qn
```

---

## 关键配置说明

### 静态链接 CRT（已配置）

`.cargo/config.toml` 已添加：
```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

效果：编译后的 `edge-agent.exe` **不依赖** `vcruntime140.dll`，在干净的 Windows 机器上也能直接运行。

### 数据库与缓存路径

Windows 上自动映射：
- 数据库：`%LOCALAPPDATA%\com.edgeagent.app\edge_agent.db`
- 快照缓存：`%LOCALAPPDATA%\com.edgeagent.app\snapshots\`

无需额外配置。

### 防火墙

首次运行会监听 `0.0.0.0:8080`，Windows 防火墙会弹窗提示，选择**允许访问**。

如需手动放行：
```powershell
# PowerShell 管理员权限
New-NetFirewallRule -DisplayName "视觉巡检智能体" -Direction Inbound -LocalPort 8080 -Protocol TCP -Action Allow
```

---

## 快速验证（Windows 上）

```powershell
# 1. 启动
.\edge-agent.exe

# 2. 浏览器访问
start http://localhost:8080

# 3. 接入 ONVIF 摄像头并测试抓图
```

---

## 常见问题

| 问题 | 原因 | 解决 |
|---|---|---|
| `edge-agent.exe` 提示缺少 DLL | 未静态链接 CRT | 确认 `.cargo/config.toml` 已配置，重新编译 |
| 前端页面空白 | `frontend/dist/` 不存在 | 先执行 `npm run build` |
| WiX 报错找不到 candle.exe | WiX 未装或未加入 PATH | 安装 WiX Toolset 并重启终端 |
| 端口 8080 被占用 | 其他程序占用 | `set BIND_ADDR=0.0.0.0:8081` 换端口 |
