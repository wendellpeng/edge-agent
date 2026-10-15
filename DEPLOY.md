# edge-agent 国产 x86 工控机部署指南

> 适用平台：兆芯 KX-U6780A / 海光 HG-3350 等国产 x86 CPU  
> 适用系统：统信 UOS 服务器版 / 麒麟 V10（Debian 系）  
> 部署目标：边缘设备 `./edge-agent` 后台常驻运行，浏览器访问 `http://<IP>:8080`

---

## 一、前置条件

### 1.1 开发机（构建端）

| 检查项      | 要求                            | 验证命令                     |
| -------- | ----------------------------- | ------------------------ |
| 操作系统     | macOS / Linux / Windows(WSL2) | —                        |
| Rust 工具链 | ≥ 1.70                        | `cargo --version`        |
| Node.js  | ≥ 18                          | `node --version`         |
| 网络       | 能访问 crates.io / npm registry  | `curl https://crates.io` |

### 1.2 目标机器（边缘端）

| 检查项    | 要求              | 验证命令                                  |
| ------ | --------------- | ------------------------------------- |
| CPU 架构 | x86_64          | `uname -m`                            |
| 操作系统   | 统信 UOS / 麒麟 V10 | `cat /etc/os-release`                 |
| 内存     | ≥ 8GB（推荐 16GB）  | `free -h`                             |
| 磁盘     | ≥ 128GB SSD     | `df -h`                               |
| 网络     | 双千兆网口已配置        | `ip addr`                             |
| OpenCV | 已安装数据文件         | `ls /usr/share/opencv4/haarcascades/` |

---

## 二、首次部署（完整流程）

### 2.1 目标机器安装系统依赖

在**边缘工控机**上执行（root 或 sudo 权限）：

```bash
# 更新源
sudo apt-get update

# 安装 OpenCV 运行时库（统信/麒麟基于 Debian，包名一致）
sudo apt-get install -y libopencv-core406 libopencv-imgproc406 libopencv-imgcodecs406 libopencv-objdetect406 opencv-data

# 如果 apt 找不到包，尝试通用名
sudo apt-get install -y libopencv-dev opencv-data

# 安装运行依赖（ SQLite 无单独包，已静态链接进二进制）
sudo apt-get install -y curl

# 创建工作目录
sudo mkdir -p /opt/edge-agent
sudo chown $(whoami) /opt/edge-agent
```

> **国产系统注意**：统信/麒麟的软件源可能滞后，若 `libopencv-*` 版本号对不上，直接装 `libopencv-dev` 即可，edge-agent 动态链接的是 `libopencv_*.so` 主版本号。

### 2.2 开发机构建（本地交叉编译）

在**你的 Mac/开发机**上执行：

```bash
cd /Users/wendellpeng/python/auto_capture/edge-agent

# Step 1: 构建前端（产物写入 frontend/dist/）
cd frontend
npm install
npm run build
cd ..

# Step 2: 构建后端（rust-embed 自动把 frontend/dist/ 内嵌进二进制）
# 目标 x86_64-unknown-linux-gnu（国产工控机架构）
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

> **macOS 交叉编译注意**：macOS 默认没有 Linux 链接器，需要安装交叉编译工具链：
>
> ```bash
> # macOS 上安装 Linux 交叉链接器
> brew install messense/macos-cross-toolchains/x86_64-unknown-linux-gnu
> # 然后在 ~/.cargo/config.toml 添加：
> [target.x86_64-unknown-linux-gnu]
> linker = "x86_64-unknown-linux-gnu-gcc"
> ```
>
> 如果交叉编译环境配起来麻烦，**更简单的方案**：直接在目标工控机上安装 Rust 后 `cargo build --release`（第一次慢，后续增量编译快）。

### 2.3 打包

```bash
# 创建部署包目录
mkdir -p deploy/edge-agent

# 复制二进制
cp target/x86_64-unknown-linux-gnu/release/edge-agent deploy/edge-agent/

# 创建 systemd 服务文件
cat > deploy/edge-agent/edge-agent.service << 'EOF'
[Unit]
Description=Edge Data Collection Agent
After=network.target

[Service]
Type=simple
User=edge
Group=edge
WorkingDirectory=/opt/edge-agent
ExecStart=/opt/edge-agent/edge-agent
Restart=always
RestartSec=5
Environment="RUST_LOG=info"
Environment="BIND_ADDR=0.0.0.0:8080"

# 资源限制（边缘设备通常资源有限）
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
EOF

# 创建安装脚本
cat > deploy/edge-agent/install.sh << 'EOF'
#!/bin/bash
set -e

APP_DIR="/opt/edge-agent"
SERVICE_FILE="/etc/systemd/system/edge-agent.service"

echo "==> 安装 edge-agent..."

# 创建用户（如果不存在）
if ! id -u edge &>/dev/null; then
    sudo useradd -r -s /bin/false edge
fi

# 复制二进制
sudo mkdir -p $APP_DIR
sudo cp edge-agent $APP_DIR/
sudo chmod +x $APP_DIR/edge-agent
sudo chown -R edge:edge $APP_DIR

# 安装 systemd 服务
sudo cp edge-agent.service $SERVICE_FILE
sudo systemctl daemon-reload
sudo systemctl enable edge-agent
sudo systemctl start edge-agent

echo "==> edge-agent 已安装并启动"
echo "==> 状态检查: sudo systemctl status edge-agent"
echo "==> 日志查看: sudo journalctl -u edge-agent -f"
echo "==> 浏览器访问: http://$(hostname -I | awk '{print $1}'):8080"
EOF
chmod +x deploy/edge-agent/install.sh

# 打成 tar.gz
cd deploy && tar czvf edge-agent-x86_64.tar.gz edge-agent/
cd ..
```

### 2.4 传输到目标机器

```bash
# 方式 A：scp 直接传（目标机器 IP 为 192.168.1.100）
scp deploy/edge-agent-x86_64.tar.gz admin@192.168.1.100:/tmp/

# 方式 B：U 盘拷贝（现场无网络时）
# 把 deploy/edge-agent-x86_64.tar.gz 拷到 U 盘，插到工控机 /mnt 下
```

### 2.5 目标机器安装

在**边缘工控机**上执行：

```bash
cd /tmp
tar xzvf edge-agent-x86_64.tar.gz
cd edge-agent
sudo ./install.sh
```

安装完成后验证：

```bash
# 查看服务状态
sudo systemctl status edge-agent

# 查看实时日志
sudo journalctl -u edge-agent -f

# 检查端口监听
ss -tlnp | grep 8080

# 本机测试
 curl http://localhost:8080
```

---

## 三、升级流程

后续版本升级只需替换二进制，数据库会自动兼容：

```bash
# 开发机构建新版本后...
scp target/x86_64-unknown-linux-gnu/release/edge-agent admin@192.168.1.100:/tmp/

# 目标机器
sudo systemctl stop edge-agent
sudo cp /tmp/edge-agent /opt/edge-agent/edge-agent
sudo chmod +x /opt/edge-agent/edge-agent
sudo systemctl start edge-agent
sudo systemctl status edge-agent
```

---

## 四、systemd 服务配置说明

| 配置项                     | 值              | 说明                  |
| ----------------------- | -------------- | ------------------- |
| `User=edge`             | 非 root 运行      | 最小权限原则，即使漏洞被利用也影响有限 |
| `Restart=always`        | 崩溃自动重启         | 边缘现场无人值守，必须自恢复      |
| `RestartSec=5`          | 5 秒后重启         | 避免崩溃后高频重启耗尽资源       |
| `LimitNOFILE=65535`     | 文件句柄上限         | 多路摄像头并发时需要          |
| `Environment=BIND_ADDR` | `0.0.0.0:8080` | 绑定所有网卡，浏览器可远程访问     |

---

## 五、防火墙配置

如果工控机开了防火墙，放行 8080 端口：

```bash
# 麒麟/统信（ufw 通常未装，直接用 iptables）
sudo iptables -I INPUT -p tcp --dport 8080 -j ACCEPT
sudo iptables-save | sudo tee /etc/iptables/rules.v4

# 或者如果装了 ufw
sudo ufw allow 8080/tcp
```

**注意**：如果摄像头和业务网分属不同网段（如 eth0 接摄像头 VLAN，eth1 接业务网），建议只在 eth1（业务网卡）上开放 8080，eth0 不开放管理端口：

```bash
# 仅 eth1（假设业务网卡为 eth1）放行 8080
sudo iptables -I INPUT -i eth1 -p tcp --dport 8080 -j ACCEPT
```

---

## 六、日志与监控

### 6.1 日志查看

```bash
# 实时跟踪
sudo journalctl -u edge-agent -f

# 查看今天日志
sudo journalctl -u edge-agent --since today

# 导出最近 1000 行
sudo journalctl -u edge-agent -n 1000 > /tmp/edge-agent.log
```

### 6.2 日志持久化（可选）

systemd 默认日志在内存中，重启后丢失。如需持久化：

```bash
sudo mkdir -p /var/log/journal
sudo systemctl restart systemd-journald
```

### 6.3 磁盘监控（快照缓存可能占空间）

```bash
# 查看快照缓存占用
du -sh ~/.cache/com.edgeagent.app/snapshots/

# 添加定时清理（保留 7 天）
echo "0 2 * * * find ~/.cache/com.edgeagent.app/snapshots/ -mtime +7 -delete" | crontab -
```

---

## 七、常见问题排查

| 现象                                           | 根因                    | 修复                                                                                |
| -------------------------------------------- | --------------------- | --------------------------------------------------------------------------------- |
| `systemctl start` 后 `failed`                 | 端口 8080 被占用           | `sudo ss -tlnp \| grep 8080` 找到占用进程，杀掉或改 `BIND_ADDR`                              |
| 日志报 `libopencv_*.so: cannot open`            | OpenCV 运行时未装          | `sudo apt-get install libopencv-dev`                                              |
| 日志报 `No such file or directory (os error 2)` | Haar Cascade XML 文件缺失 | `sudo apt-get install opencv-data`，或手动复制 XML 到 `/usr/share/opencv4/haarcascades/` |
| 浏览器访问不了                                      | 防火墙拦截                 | `sudo iptables -I INPUT -p tcp --dport 8080 -j ACCEPT`                            |
| 摄像头发现为空                                      | ONVIF 未开启 / 网段隔离      | 检查摄像头后台开启 ONVIF；确认工控机与摄像头同网段                                                      |
| 人脸检测占用 CPU 高                                 | Haar Cascade 算法较重     | 正常（单帧约 50-200ms），后续可换 ONNX 轻量模型                                                   |
| 服务崩溃后没自动重启                                   | systemd 配置未生效         | `sudo systemctl daemon-reload && sudo systemctl enable edge-agent`                |

---

## 八、部署检查清单（现场实施用）

- [ ] 工控机已安装统信/麒麟，双网口已配置 IP
- [ ] OpenCV 运行时已安装（`pkg-config --modversion opencv4` 有输出）
- [ ] edge-agent 二进制已复制到 `/opt/edge-agent/`
- [ ] systemd 服务已启用（`systemctl is-enabled edge-agent` 输出 `enabled`）
- [ ] 服务状态为 `active (running)`
- [ ] 浏览器访问 `http://<工控机IP>:8080` 能打开管理页
- [ ] 防火墙 8080 端口已放行
- [ ] 摄像头 IP 能 ping 通（`ping <摄像头IP>`）
- [ ] 摄像头 ONVIF 已在网页后台开启
