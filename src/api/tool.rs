use actix_web::{delete, get, post, put, web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use crate::{
    db::db_path,
    models::{ApiResponse, CameraDiscovery, Tool},
    strix,
};
use chrono::Local;
use rusqlite::Connection;
use std::sync::Arc;

#[get("/api/tools")]
pub async fn list_tools() -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    let mut stmt = conn.prepare("SELECT * FROM tools ORDER BY created_at DESC")
        .map_err(|e| ApiError(e.to_string()))?;

    let tools = stmt.query_map([], |row| {
        Ok(Tool {
            id: row.get("id")?,
            tool_type: row.get("tool_type")?,
            endpoint: row.get("endpoint")?,
            xaddr: row.get("xaddr")?,
            name: row.get("name")?,
            location: row.get("location")?,
            username: row.get("username")?,
            password: row.get("password")?,
            channel: row.get("channel").unwrap_or(1),
            brand: row.get("brand").ok(),
            enabled: row.get::<_, i32>("enabled")? == 1,
            created_at: row.get("created_at")?,
        })
    }).map_err(|e| ApiError(e.to_string()))?
    .collect::<Result<Vec<_>, _>>().map_err(|e| ApiError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse::ok(tools)))
}

#[post("/api/tools")]
pub async fn add_tool(body: web::Json<Tool>) -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    conn.execute(
        "INSERT INTO tools (tool_type, name, endpoint, xaddr, location, username, password, channel, brand, enabled, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            body.tool_type, body.name, body.endpoint, body.xaddr,
            body.location, body.username, body.password,
            body.channel.max(1),
            body.brand,
            if body.enabled { 1 } else { 0 },
            Local::now().to_rfc3339(),
        ],
    ).map_err(|e| ApiError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(conn.last_insert_rowid())))
}

#[put("/api/tools/{id}")]
pub async fn update_tool(path: web::Path<i64>, body: web::Json<Tool>) -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    conn.execute(
        "UPDATE tools SET tool_type=?1, name=?2, endpoint=?3, xaddr=?4, location=?5, username=?6, password=?7, channel=?8, brand=?9, enabled=?10 WHERE id=?11",
        rusqlite::params![
            body.tool_type, body.name, body.endpoint, body.xaddr,
            body.location, body.username, body.password,
            body.channel.max(1),
            body.brand,
            if body.enabled { 1 } else { 0 },
            *path,
        ],
    ).map_err(|e| ApiError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok("updated")))
}

#[delete("/api/tools/{id}")]
pub async fn delete_tool(path: web::Path<i64>) -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    conn.execute("DELETE FROM tools WHERE id = ?1", [*path])
        .map_err(|e| ApiError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok("deleted")))
}

// ===================== Strix URL 模板 =====================

#[get("/api/tools/rtsp-templates")]
pub async fn list_rtsp_templates() -> Result<HttpResponse> {
    let patterns = strix::get_patterns();
    Ok(HttpResponse::Ok().json(ApiResponse::ok(patterns)))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RtspUrlRequest {
    pub pattern_name: String,
    pub ip: String,
    pub username: String,
    pub password: String,
    pub channel: i32,
}

#[post("/api/tools/rtsp-url")]
pub async fn generate_rtsp_url(body: web::Json<RtspUrlRequest>) -> Result<HttpResponse> {
    let patterns = strix::get_patterns();
    let pattern = patterns.iter()
        .find(|p| p.name == body.pattern_name)
        .ok_or_else(|| ApiError("未知模式".to_string()))?;

    let ch = body.channel.max(1);
    let url = pattern.template
        .replace("{user}", &body.username)
        .replace("{pass}", &body.password)
        .replace("{ip}", &body.ip)
        .replace("{channel}", &ch.to_string());

    Ok(HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
        "rtsp_url": url,
        "pattern_name": body.pattern_name,
        "channel": ch,
    }))))
}

// ===================== 局域网扫描（仅 RTSP 554） =====================

#[get("/api/tools/discover")]
pub async fn discover_tools(query: web::Query<std::collections::HashMap<String, String>>) -> Result<HttpResponse> {
    let timeout_secs: u64 = query.get("timeout")
        .and_then(|s| s.parse().ok())
        .unwrap_or(5)
        .max(1)
        .min(30);

    log::info!("开始 RTSP 554 端口扫描，超时: {} 秒...", timeout_secs);

    // 只扫描 RTSP 554 端口
    let devices = scan_rtsp_554(timeout_secs).await;

    log::info!("扫描完成，发现 {} 台设备", devices.len());

    if devices.is_empty() {
        log::warn!("未扫描到 RTSP 设备。可能原因: 1) 设备未开启 RTSP; 2) 不在同一网段; 3) 防火墙拦截 554 端口");
    }

    Ok(HttpResponse::Ok().json(ApiResponse::ok(devices)))
}

/// 仅扫描 RTSP 554 端口，返回开放 554 的设备列表
async fn scan_rtsp_554(timeout_secs: u64) -> Vec<CameraDiscovery> {
    let local_ips = match local_ip_address::list_afinet_netifas() {
        Ok(ips) => ips.into_iter()
            .filter(|(_, ip)| ip.is_ipv4() && !ip.is_loopback())
            .map(|(_, ip)| ip)
            .collect::<Vec<_>>(),
        Err(e) => {
            log::warn!("获取本地网络接口失败: {}", e);
            return Vec::new();
        }
    };

    if local_ips.is_empty() {
        log::warn!("未找到本地非回环 IPv4 地址，跳过扫描");
        return Vec::new();
    }

    let semaphore = Arc::new(tokio::sync::Semaphore::new(100));
    let _scan_timeout = std::time::Duration::from_secs(timeout_secs);
    let mut all_results = Vec::new();

    for ip in local_ips {
        let ip_str = ip.to_string();
        let segments: Vec<&str> = ip_str.split('.').collect();
        if segments.len() != 4 { continue; }
        let prefix = format!("{}.{}.{}", segments[0], segments[1], segments[2]);

        log::info!("[RTSP扫描] 网段: {}.1-254，端口 554", prefix);

        // TCP 连接 554 端口
        let mut handles = Vec::new();
        for i in 1u8..=254 {
            let addr = format!("{}.{}", prefix, i);
            let sem = semaphore.clone();
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                let socket_addr = std::net::SocketAddr::new(
                    std::net::IpAddr::V4(addr.parse().ok()?),
                    554,
                );
                match tokio::time::timeout(
                    std::time::Duration::from_millis(500),
                    tokio::net::TcpStream::connect(socket_addr)
                ).await {
                    Ok(Ok(_)) => Some(addr),
                    _ => None,
                }
            });
            handles.push(handle);
        }

        // 收集开放的 IP
        let mut open_ips: Vec<String> = Vec::new();
        for h in handles {
            if let Ok(Some(ip)) = h.await {
                open_ips.push(ip);
            }
        }

        log::info!("[RTSP扫描] {} 个 IP 的 554 端口开放", open_ips.len());

        // RTSP OPTIONS 验证
        let mut verify_handles = Vec::new();
        for target_ip in open_ips {
            let sem = semaphore.clone();
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                match rtsp_options_probe(&target_ip, 554).await {
                    Ok(true) => {
                        log::info!("[RTSP] 验证通过: {}:554", target_ip);
                        Some(CameraDiscovery {
                            endpoint: format!("rtsp://{}:554/", target_ip),
                            xaddr: Some(format!("rtsp://{}:554/", target_ip)),
                            name: Some(format!("RTSP 设备 ({})", target_ip)),
                            scopes: vec![],
                            types: vec![],
                            source: "rtsp_scan".to_string(),
                        })
                    }
                    _ => {
                        log::debug!("[RTSP] OPTIONS 验证失败: {}:554", target_ip);
                        None
                    }
                }
            });
            verify_handles.push(handle);
        }

        for h in verify_handles {
            if let Ok(Some(discovery)) = h.await {
                all_results.push(discovery);
            }
        }
    }

    all_results
}

/// RTSP OPTIONS 探测
async fn rtsp_options_probe(ip: &str, port: u16) -> Result<bool, String> {
    let addr = format!("{}:{}", ip, port);
    let stream = tokio::net::TcpStream::connect(&addr).await
        .map_err(|e| format!("TCP 连接失败: {}", e))?;

    let rtsp_request = format!(
        "OPTIONS rtsp://{}:{}/ RTSP/1.0\r\nCSeq: 1\r\nUser-Agent: EdgeAgent/1.0\r\n\r\n",
        ip, port
    );

    let (mut reader, mut writer) = stream.into_split();
    use tokio::io::{AsyncWriteExt, AsyncReadExt};

    writer.write_all(rtsp_request.as_bytes()).await
        .map_err(|e| format!("发送 RTSP 请求失败: {}", e))?;
    writer.flush().await.map_err(|e| format!("flush 失败: {}", e))?;
    drop(writer);

    let mut buf = vec![0u8; 1024];
    let n = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        reader.read(&mut buf)
    ).await.map_err(|_| "读取超时".to_string())?
        .map_err(|e| format!("读取失败: {}", e))?;

    let response = String::from_utf8_lossy(&buf[..n]);
    Ok(response.contains("RTSP/1.0"))
}

// ===================== 抓图 =====================

/// 从摄像头抓取真实图片，返回字节数组
/// 支持 ONVIF 和 RTSP 两种方式
pub async fn fetch_snapshot(tool: &Tool) -> Result<Vec<u8>, String> {
    match tool.tool_type.as_str() {
        "rtsp_camera" => fetch_snapshot_rtsp(tool).await,
        _ => fetch_snapshot_onvif(tool).await,
    }
}

async fn fetch_snapshot_onvif(tool: &Tool) -> Result<Vec<u8>, String> {
    let user = tool.username.as_deref().unwrap_or("");
    let pass = tool.password.as_deref().unwrap_or("");

    let mut builder = oxvif::OnvifSession::builder(&tool.endpoint);
    if !user.is_empty() {
        builder = builder.with_credentials(user, pass);
    }
    let session = builder.build().await
        .map_err(|e| format!("ONVIF 连接失败: {}", e))?;

    let profiles = session.get_profiles().await
        .map_err(|e| format!("获取 Media Profile 失败: {}", e))?;

    let idx = (tool.channel.max(1) - 1) as usize;
    let profile = profiles.get(idx)
        .or_else(|| profiles.first())
        .ok_or_else(|| "设备没有可用 Media Profile".to_string())?;

    log::info!("工具 [{}] 使用 Profile {}/{} (token={})", tool.name, idx + 1, profiles.len(), profile.token);

    let snap = session.get_snapshot_uri(&profile.token).await
        .map_err(|e| format!("获取 Snapshot URI 失败: {}", e))?;

    let client = reqwest::Client::new();
    let resp = client.get(&snap.uri)
        .timeout(std::time::Duration::from_secs(30))
        .send().await
        .map_err(|e| format!("下载图片失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("下载图片返回 HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().await
        .map_err(|e| format!("读取图片数据失败: {}", e))?;
    Ok(bytes.to_vec())
}

async fn fetch_snapshot_rtsp(tool: &Tool) -> Result<Vec<u8>, String> {
    let rtsp_url = &tool.endpoint;
    log::info!("RTSP 抓图: {}", rtsp_url);

    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("com.edgeagent.app/snapshots");
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("创建缓存目录失败: {}", e))?;

    let filename = format!("rtsp_{}_{}.jpg", tool.id, Local::now().format("%Y%m%d_%H%M%S"));
    let path = cache_dir.join(&filename);

    // 使用 ffmpeg 截取一帧
    let output = tokio::process::Command::new("ffmpeg")
        .args(&[
            "-y",
            "-rtsp_transport", "tcp",
            "-i", rtsp_url,
            "-vframes", "1",
            "-q:v", "2",
            "-f", "image2",
            path.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output().await
        .map_err(|e| format!("ffmpeg 执行失败: {}。请确认系统已安装 ffmpeg", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg 截图失败: {}", stderr));
    }

    let bytes = std::fs::read(&path)
        .map_err(|e| format!("读取截图文件失败: {}", e))?;

    // 清理临时文件
    std::fs::remove_file(&path).ok();

    Ok(bytes)
}

#[post("/api/tools/{id}/snapshot")]
pub async fn test_snapshot(path: web::Path<i64>) -> Result<HttpResponse> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    let tool: Tool = conn.query_row(
        "SELECT * FROM tools WHERE id = ?1",
        [*path],
        |row| {
            Ok(Tool {
                id: row.get("id")?,
                tool_type: row.get("tool_type")?,
                endpoint: row.get("endpoint")?,
                xaddr: row.get("xaddr")?,
                name: row.get("name")?,
                location: row.get("location")?,
                username: row.get("username")?,
                password: row.get("password")?,
                channel: row.get("channel").unwrap_or(1),
                brand: row.get("brand").ok(),
                enabled: row.get::<_, i32>("enabled")? == 1,
                created_at: row.get("created_at")?,
            })
        },
    ).map_err(|e| ApiError(format!("加载工具失败: {}", e)))?;

    let bytes = fetch_snapshot(&tool).await
        .map_err(|e| ApiError(format!("抓图失败: {}", e)))?;
    let b64 = STANDARD.encode(&bytes);

    Ok(HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
        "success": true,
        "image_base64": b64,
        "tool_name": tool.name,
    }))))
}

use crate::api::ApiError;

// ===================== NVR 通道枚举 =====================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChannelInfo {
    pub index: i32,
    pub token: String,
    pub name: Option<String>,
}

#[get("/api/tools/{id}/channels")]
pub async fn list_channels(path: web::Path<i64>) -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    let tool: Tool = conn.query_row(
        "SELECT * FROM tools WHERE id = ?1",
        [*path],
        |row| {
            Ok(Tool {
                id: row.get("id")?,
                tool_type: row.get("tool_type")?,
                endpoint: row.get("endpoint")?,
                xaddr: row.get("xaddr")?,
                name: row.get("name")?,
                location: row.get("location")?,
                username: row.get("username")?,
                password: row.get("password")?,
                channel: row.get("channel").unwrap_or(1),
                brand: row.get("brand").ok(),
                enabled: row.get::<_, i32>("enabled")? == 1,
                created_at: row.get("created_at")?,
            })
        },
    ).map_err(|e| ApiError(format!("加载工具失败: {}", e)))?;

    // RTSP 摄像头不支持通道枚举
    if tool.tool_type == "rtsp_camera" {
        return Ok(HttpResponse::Ok().json(ApiResponse::ok(vec![ChannelInfo {
            index: 1,
            token: "rtsp_main".to_string(),
            name: Some("RTSP 主码流".to_string()),
        }])));
    }

    let user = tool.username.as_deref().unwrap_or("");
    let pass = tool.password.as_deref().unwrap_or("");

    let mut builder = oxvif::OnvifSession::builder(&tool.endpoint);
    if !user.is_empty() {
        builder = builder.with_credentials(user, pass);
    }
    let session = builder.build().await
        .map_err(|e| ApiError(format!("ONVIF 连接失败: {}", e)))?;

    let profiles = session.get_profiles().await
        .map_err(|e| ApiError(format!("获取 Media Profile 失败: {}", e)))?;

    let channels: Vec<ChannelInfo> = profiles.into_iter().enumerate().map(|(i, p)| {
        ChannelInfo {
            index: (i + 1) as i32,
            token: p.token,
            name: None,
        }
    }).collect();

    Ok(HttpResponse::Ok().json(ApiResponse::ok(channels)))
}

// ===================== 设备接入向导 =====================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProbeRequest {
    pub ip: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProbeResult {
    pub ip: String,
    pub brand: String,  // 识别出的品牌
    pub channels: Vec<ProbeChannelResult>,
    pub probe_log: Vec<ProbeLogEntry>, // 阶段1的所有尝试记录
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProbeLogEntry {
    pub pattern_name: String,
    pub url: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProbeChannelResult {
    pub channel: i32,
    pub url: String,
    pub success: bool,
    pub snapshot_base64: Option<String>,
    pub error: Option<String>,
}

/// 用 ffmpeg 验证单个 RTSP URL 能否取图，先尝试 TCP 再尝试 UDP
/// 返回 (success, base64, error)
async fn verify_rtsp_url(url: &str, channel: i32) -> (bool, Option<String>, Option<String>) {
    // 先尝试 TCP
    let result = verify_rtsp_with_transport(url, channel, "tcp").await;
    if result.0 {
        return result;
    }

    // TCP 失败，尝试 UDP
    log::info!("TCP 失败，尝试 UDP: {}", url);
    let udp_result = verify_rtsp_with_transport(url, channel, "udp").await;
    if udp_result.0 {
        return udp_result;
    }

    // 都失败，返回 TCP 的错误（通常更有参考价值）
    result
}

async fn verify_rtsp_with_transport(url: &str, channel: i32, transport: &str) -> (bool, Option<String>, Option<String>) {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("com.edgeagent.app/snapshots");
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        return (false, None, Some(format!("创建缓存目录失败: {}", e)));
    }

    let filename = format!("probe_{}_{}_{}.jpg", channel, transport, Local::now().format("%H%M%S%3f"));
    let path = cache_dir.join(&filename);

    let output = match tokio::process::Command::new("ffmpeg")
        .args(&[
            "-y",
            "-rtsp_transport", transport,
            "-timeout", "10000000", // 10秒连接超时（microseconds，ffmpeg 8.0+ 用 timeout）
            "-i", url,
            "-vframes", "1",
            "-q:v", "2",
            "-f", "image2",
            path.to_str().unwrap_or("/tmp/probe.jpg"),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output().await
    {
        Ok(o) => o,
        Err(e) => return (false, None, Some(format!("ffmpeg 执行失败: {}", e))),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        std::fs::remove_file(&path).ok();
        // 提取关键错误信息（ffmpeg 输出很长，只取关键行）
        let summary = extract_ffmpeg_error(&stderr);
        return (false, None, Some(format!("[{}] {}", transport.to_uppercase(), summary)));
    }

    match std::fs::read(&path) {
        Ok(bytes) => {
            let b64 = STANDARD.encode(&bytes);
            std::fs::remove_file(&path).ok();
            (true, Some(b64), None)
        }
        Err(e) => {
            std::fs::remove_file(&path).ok();
            (false, None, Some(format!("读取截图失败: {}", e)))
        }
    }
}

/// 从 ffmpeg stderr 中提取关键错误信息
fn extract_ffmpeg_error(stderr: &str) -> String {
    let lines: Vec<&str> = stderr.lines().collect();

    // 优先查找这些关键错误模式
    for line in &lines {
        let lower = line.to_lowercase();
        if lower.contains("unauthorized") || lower.contains("401") {
            return "认证失败：账号或密码错误".to_string();
        }
        if lower.contains("connection refused") {
            return "连接被拒绝：端口未开放或服务未启动".to_string();
        }
        if lower.contains("connection timed out") || lower.contains("timeout") {
            return "连接超时：设备无响应".to_string();
        }
        if lower.contains("no route to host") || lower.contains("network is unreachable") {
            return "网络不可达：设备离线或不在同一网段".to_string();
        }
        if lower.contains("invalid data") || lower.contains("not found") {
            return "URL 无效：通道或路径不存在".to_string();
        }
    }

    // 没找到关键模式，返回前 200 字符
    stderr.chars().take(200).collect::<String>().trim().to_string()
}

#[post("/api/tools/probe")]
pub async fn probe_device(body: web::Json<ProbeRequest>) -> Result<HttpResponse> {
    let ip = &body.ip;
    let username = &body.username;
    let password = &body.password;

    log::info!("开始探测设备: {}，账号: {}", ip, username);

    // 阶段0：HTTP 辅助探测 — 通过 Web 接口识别品牌和型号
    let http_brand = http_probe_brand(ip).await;
    if let Some(ref b) = http_brand {
        log::info!("HTTP 辅助探测识别到品牌: {}", b);
    }

    // 阶段1：并发尝试所有 Strix 模式的通道 1-4，确定品牌和 URL 模式
    // 不能只试通道1，因为NVR的通道1可能恰好没有接摄像头（离线）
    let patterns = strix::get_patterns();
    let mut detected_pattern: Option<String> = None;

    // 如果 HTTP 识别出品牌，优先尝试该品牌的模式
    let ordered_patterns: Vec<_> = if let Some(ref brand) = http_brand {
        let mut ordered = Vec::new();
        let brand_lower = brand.to_lowercase();
        for p in &patterns {
            let p_brand = strix::guess_brand_from_pattern(&p.name).to_lowercase();
            if p_brand == brand_lower || p.name.to_lowercase().contains(&brand_lower) {
                ordered.push(p.clone());
            }
        }
        // 再把其他模式追加到后面
        for p in &patterns {
            let p_brand = strix::guess_brand_from_pattern(&p.name).to_lowercase();
            if p_brand != brand_lower && !p.name.to_lowercase().contains(&brand_lower) {
                ordered.push(p.clone());
            }
        }
        ordered
    } else {
        patterns.clone()
    };

    log::info!("阶段1：并发尝试 {} 种 Strix URL 模式的通道 1-4...", ordered_patterns.len());

    let semaphore = Arc::new(tokio::sync::Semaphore::new(8));
    let mut handles = Vec::new();

    for pattern in ordered_patterns.iter() {
        let pattern_name = pattern.name.clone();
        let template = pattern.template.clone();
        let sem = semaphore.clone();
        let user = username.to_string();
        let pass = password.to_string();
        let target_ip = ip.to_string();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.ok()?;
            let mut log_entries: Vec<ProbeLogEntry> = Vec::new();

            // 尝试通道 1, 2, 3, 4
            for ch in [1, 2, 3, 4] {
                let url = template
                    .replace("{user}", &user)
                    .replace("{pass}", &pass)
                    .replace("{ip}", &target_ip)
                    .replace("{channel}", &ch.to_string());

                log::debug!("尝试模式 [{}] 通道 {}: {}", pattern_name, ch, url);
                let (success, _snapshot, error) = verify_rtsp_url(&url, ch).await;

                log_entries.push(ProbeLogEntry {
                    pattern_name: format!("{} (ch{})", pattern_name, ch),
                    url: url.clone(),
                    success,
                    error: error.clone(),
                });

                if success {
                    log::info!("✅ 模式识别成功: {} 通道 {}", pattern_name, ch);
                    return Some((pattern_name, log_entries, true));
                }
            }

            // 所有通道都失败
            Some(("".to_string(), log_entries, false))
        });
        handles.push(handle);
    }

    // 收集所有尝试结果
    let mut probe_log: Vec<ProbeLogEntry> = Vec::new();
    for handle in handles {
        if let Ok(Some((name, entries, success))) = handle.await {
            probe_log.extend(entries);
            if success && detected_pattern.is_none() && !name.is_empty() {
                detected_pattern = Some(name);
            }
        }
    }

    // 按模式名排序日志
    probe_log.sort_by(|a, b| a.pattern_name.cmp(&b.pattern_name));

    let (brand, main_pattern) = match detected_pattern {
        Some(pattern_name) => {
            let brand = strix::guess_brand_from_pattern(&pattern_name);
            // 找到该品牌的主码流模板
            let main = ordered_patterns.iter()
                .find(|p| p.name == pattern_name || (strix::guess_brand_from_pattern(&p.name) == brand && p.stream_type == "main"))
                .cloned();
            (brand, main)
        }
        None => {
            log::warn!("所有 Strix URL 模式的通道 1-4 均验证失败");
            return Ok(HttpResponse::Ok().json(ApiResponse::ok(ProbeResult {
                ip: ip.clone(),
                brand: "unknown".to_string(),
                channels: vec![ProbeChannelResult {
                    channel: 1,
                    url: "".to_string(),
                    success: false,
                    snapshot_base64: None,
                    error: Some("无法识别设备，请检查：1) 账号密码是否正确；2) RTSP是否开启；3) 通道1-4是否均离线".to_string()),
                }],
                probe_log,
            })));
        }
    };

    // 阶段2：用识别出的模式验证通道 1-16
    let tmpl = match main_pattern {
        Some(p) => p,
        None => {
            return Ok(HttpResponse::Ok().json(ApiResponse::ok(ProbeResult {
                ip: ip.clone(),
                brand,
                channels: vec![ProbeChannelResult {
                    channel: 1,
                    url: "".to_string(),
                    success: false,
                    snapshot_base64: None,
                    error: Some("未找到主码流模板".to_string()),
                }],
                probe_log,
            })));
        }
    };

    log::info!("阶段2：用模式 '{}' 验证通道 1-16...", tmpl.name);

    let semaphore2 = Arc::new(tokio::sync::Semaphore::new(4));
    let mut handles2 = Vec::new();

    for ch in 1..=16 {
        let url = tmpl.template
            .replace("{user}", username)
            .replace("{pass}", password)
            .replace("{ip}", ip)
            .replace("{channel}", &ch.to_string());
        let sem = semaphore2.clone();
        let ch_i32 = ch as i32;

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.ok()?;
            log::info!("验证通道 {}: {}", ch_i32, url);
            let (success, snapshot, error) = verify_rtsp_url(&url, ch_i32).await;
            Some(ProbeChannelResult {
                channel: ch_i32,
                url,
                success,
                snapshot_base64: snapshot,
                error,
            })
        });
        handles2.push(handle);
    }

    let mut channels = Vec::new();
    for h in handles2 {
        if let Ok(Some(result)) = h.await {
            channels.push(result);
        }
    }

    // 按通道排序
    channels.sort_by_key(|c| c.channel);

    let success_count = channels.iter().filter(|c| c.success).count();
    log::info!("探测完成: {} 通道成功 / 16 通道", success_count);

    Ok(HttpResponse::Ok().json(ApiResponse::ok(ProbeResult {
        ip: ip.clone(),
        brand,
        channels,
        probe_log,
    })))
}

/// HTTP 辅助探测：通过设备的 Web 接口识别品牌
async fn http_probe_brand(ip: &str) -> Option<String> {
    let ports = vec![80u16, 8000, 8080];
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return None,
    };

    for port in ports {
        let url = format!("http://{}:{}/", ip, port);
        if let Ok(resp) = client.get(&url).send().await {
            // 检查 Server 头
            if let Some(server) = resp.headers().get("server").and_then(|v| v.to_str().ok()) {
                let s = server.to_lowercase();
                if s.contains("hikvision") || s.contains("dvr") || s.contains("nvr") {
                    return Some("hikvision".to_string());
                }
                if s.contains("dahua") {
                    return Some("dahua".to_string());
                }
                if s.contains("tp-link") || s.contains("tplink") {
                    return Some("tp-link".to_string());
                }
                if s.contains("axis") {
                    return Some("axis".to_string());
                }
                if s.contains("bosch") {
                    return Some("bosch".to_string());
                }
            }

            // 检查响应体中的品牌标识
            if let Ok(text) = resp.text().await {
                let t = text.to_lowercase();
                if t.contains("hikvision") || t.contains("海康威视") {
                    return Some("hikvision".to_string());
                }
                if t.contains("dahua") || t.contains("大华") {
                    return Some("dahua".to_string());
                }
                if t.contains("ezviz") || t.contains("萤石") {
                    return Some("ezviz".to_string());
                }
                if t.contains("uniview") || t.contains("unv") || t.contains("宇视") {
                    return Some("uniview".to_string());
                }
                if t.contains("tp-link") || t.contains("tplink") {
                    return Some("tp-link".to_string());
                }
                if t.contains("imou") || t.contains("乐橙") {
                    return Some("imou".to_string());
                }
                if t.contains("xiaomi") || t.contains("小米") || t.contains("米家") {
                    return Some("xiaomi".to_string());
                }
                if t.contains("huawei") || t.contains("华为") || t.contains("hilook") {
                    return Some("huawei".to_string());
                }
                if t.contains("reolink") {
                    return Some("reolink".to_string());
                }
                if t.contains("foscam") {
                    return Some("foscam".to_string());
                }
                if t.contains("amcrest") {
                    return Some("amcrest".to_string());
                }
            }
        }
    }

    None
}

/// 手动验证单个 RTSP URL
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerifyUrlRequest {
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerifyUrlResult {
    pub url: String,
    pub success: bool,
    pub snapshot_base64: Option<String>,
    pub error: Option<String>,
}

#[post("/api/tools/verify-url")]
pub async fn verify_rtsp_url_endpoint(body: web::Json<VerifyUrlRequest>) -> Result<HttpResponse> {
    let url = &body.url;
    log::info!("手动验证 RTSP URL: {}", url);

    let (success, snapshot, error) = verify_rtsp_url(url, 1).await;

    Ok(HttpResponse::Ok().json(ApiResponse::ok(VerifyUrlResult {
        url: url.clone(),
        success,
        snapshot_base64: snapshot,
        error,
    })))
}
