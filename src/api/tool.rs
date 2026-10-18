use actix_web::{delete, get, post, put, web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use crate::{
    db::db_path,
    models::{ApiResponse, CameraDiscovery, Tool, rtsp_templates},
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

// ===================== 品牌模板 =====================

#[get("/api/tools/rtsp-templates")]
pub async fn list_rtsp_templates() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(ApiResponse::ok(rtsp_templates())))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RtspUrlRequest {
    pub brand: String,
    pub ip: String,
    pub username: String,
    pub password: String,
    pub channel: i32,
}

#[post("/api/tools/rtsp-url")]
pub async fn generate_rtsp_url(body: web::Json<RtspUrlRequest>) -> Result<HttpResponse> {
    let templates = rtsp_templates();
    let tmpl = templates.iter()
        .find(|t| t.brand == body.brand)
        .ok_or_else(|| ApiError("未知品牌".to_string()))?;

    let ch = body.channel.max(1);
    let url = tmpl.main_stream
        .replace("{user}", &body.username)
        .replace("{pass}", &body.password)
        .replace("{ip}", &body.ip)
        .replace("{channel}", &ch.to_string());

    Ok(HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
        "rtsp_url": url,
        "brand": body.brand,
        "channel": ch,
    }))))
}

// ===================== 局域网扫描 =====================

#[get("/api/tools/discover")]
pub async fn discover_tools(query: web::Query<std::collections::HashMap<String, String>>) -> Result<HttpResponse> {
    let timeout_secs: u64 = query.get("timeout")
        .and_then(|s| s.parse().ok())
        .unwrap_or(5)
        .max(1)
        .min(30);

    let mut result = Vec::new();

    // 1. 主路径：ONVIF WS-Discovery UDP 多播
    log::info!("开始 ONVIF WS-Discovery 扫描，超时: {} 秒...", timeout_secs);
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let devices = oxvif::discovery::probe(timeout).await;
    log::info!("WS-Discovery 扫描完成，发现 {} 台设备", devices.len());

    for d in &devices {
        let name = d.scopes.iter()
            .find(|s| s.contains("/name/"))
            .and_then(|s| s.split("/name/").nth(1))
            .map(|n| n.to_string());

        log::debug!("WS-Discovery 发现: endpoint={}, xaddr={:?}, name={:?}", d.endpoint, d.xaddrs.first(), name);

        result.push(CameraDiscovery {
            endpoint: d.endpoint.clone(),
            xaddr: d.xaddrs.clone().into_iter().next(),
            name,
            scopes: d.scopes.clone(),
            types: d.types.clone(),
            source: "ws_discovery".to_string(),
        });
    }

    // 2. Fallback：TCP 端口扫描（WS-Discovery 扫不到时兜底）
    if result.is_empty() {
        log::info!("WS-Discovery 未发现设备，启动 TCP 端口扫描兜底...");
        let fallback = scan_network_fallback().await;
        log::info!("TCP 端口扫描完成，发现 {} 台设备", fallback.len());
        result.extend(fallback);
    }

    if result.is_empty() {
        log::warn!("所有扫描方式均未发现设备。可能原因: 1) 摄像头未开启 ONVIF/WS-Discovery; 2) 摄像头与当前设备不在同一网段; 3) macOS 防火墙拦截了 UDP 多播或 TCP 扫描");
    }

    Ok(HttpResponse::Ok().json(ApiResponse::ok(result)))
}

/// TCP 端口扫描 + URL 验证兜底
async fn scan_network_fallback() -> Vec<CameraDiscovery> {
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
        log::warn!("未找到本地非回环 IPv4 地址，跳过端口扫描");
        return Vec::new();
    }

    let ports = vec![80u16, 8000, 8080, 554];
    let semaphore = Arc::new(tokio::sync::Semaphore::new(50));
    let mut all_results = Vec::new();

    for ip in local_ips {
        let ip_str = ip.to_string();
        let segments: Vec<&str> = ip_str.split('.').collect();
        if segments.len() != 4 { continue; }
        let prefix = format!("{}.{}.{}", segments[0], segments[1], segments[2]);

        log::info!("端口扫描网段: {}.1-254，端口 {:?}", prefix, ports);

        // 第1步：TCP 端口扫描（快速过滤）
        let mut tcp_handles = Vec::new();
        for i in 1u8..=254 {
            for port in &ports {
                let addr = format!("{}.{}", prefix, i);
                let port = *port;
                let sem = semaphore.clone();
                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.ok()?;
                    let socket_addr = std::net::SocketAddr::new(
                        std::net::IpAddr::V4(addr.parse().ok()?),
                        port,
                    );
                    match tokio::time::timeout(
                        std::time::Duration::from_millis(300),
                        tokio::net::TcpStream::connect(socket_addr)
                    ).await {
                        Ok(Ok(_)) => Some((addr, port)),
                        _ => None,
                    }
                });
                tcp_handles.push(handle);
            }
        }

        // 收集 TCP 开放的结果
        let mut open_addrs: Vec<(String, u16)> = Vec::new();
        for h in tcp_handles {
            if let Ok(Some(pair)) = h.await {
                open_addrs.push(pair);
            }
        }

        log::info!("TCP 端口扫描完成，{} 个开放端口待验证", open_addrs.len());

        // 分离 RTSP (554) 和 HTTP 端口
        let rtsp_addrs: Vec<(String, u16)> = open_addrs.iter()
            .filter(|(_, p)| *p == 554)
            .cloned()
            .collect();
        let http_addrs: Vec<(String, u16)> = open_addrs.iter()
            .filter(|(_, p)| *p != 554)
            .cloned()
            .collect();

        // 第2步A：RTSP 验证（对 554 端口发送 OPTIONS）
        let mut rtsp_verify = Vec::new();
        for (ip, port) in rtsp_addrs {
            let sem = semaphore.clone();
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                let url = format!("rtsp://{}:{}/", ip, port);
                match rtsp_options_probe(&ip, port).await {
                    Ok(true) => {
                        log::info!("RTSP 验证通过: {}", url);
                        Some((ip, port, url))
                    }
                    _ => {
                        log::debug!("RTSP 验证排除: {}", url);
                        None
                    }
                }
            });
            rtsp_verify.push(handle);
        }

        for h in rtsp_verify {
            if let Ok(Some((ip, port, endpoint))) = h.await {
                all_results.push(CameraDiscovery {
                    endpoint: endpoint.clone(),
                    xaddr: Some(endpoint),
                    name: Some(format!("RTSP 设备 ({}:{})", ip, port)),
                    scopes: vec![],
                    types: vec![],
                    source: "rtsp_scan".to_string(),
                });
            }
        }

        // 第2步B：SOAP 验证（对 HTTP 端口）
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let soap_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <s:Body>
    <tds:GetDeviceInformation/>
  </s:Body>
</s:Envelope>"#;

        let mut verify_handles = Vec::new();
        for (ip, port) in http_addrs {
            let url = format!("http://{}:{}/onvif/device_service", ip, port);
            let sem = semaphore.clone();
            let c = client.clone();
            let body = soap_body.to_string();
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                match c.post(&url)
                    .header("Content-Type", "application/soap+xml; charset=utf-8")
                    .header("SOAPAction", "http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation")
                    .body(body)
                    .send().await
                {
                    Ok(resp) => {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        let is_soap = text.contains("Envelope") || text.contains("soap") || text.contains("Body");

                        if is_soap {
                            log::info!("SOAP 验证通过: {} (HTTP {}, 响应含 SOAP)", url, status);
                            Some((ip, port, url, true))
                        } else {
                            log::debug!("SOAP 验证排除: {} (HTTP {}, 非 SOAP 响应)", url, status);
                            None
                        }
                    }
                    Err(e) => {
                        log::debug!("SOAP 请求失败: {} — {}", url, e);
                        None
                    }
                }
            });
            verify_handles.push(handle);
        }

        for h in verify_handles {
            if let Ok(Some((ip, port, endpoint, _confirmed))) = h.await {
                all_results.push(CameraDiscovery {
                    endpoint: endpoint.clone(),
                    xaddr: Some(endpoint),
                    name: Some(format!("ONVIF 设备 ({}:{})", ip, port)),
                    scopes: vec![],
                    types: vec![],
                    source: "port_scan".to_string(),
                });
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
