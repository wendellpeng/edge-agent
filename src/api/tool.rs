use actix_web::{delete, get, post, put, web, HttpResponse, Result};
use crate::{
    db::db_path,
    models::{ApiResponse, CameraDiscovery, Tool},
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
        "INSERT INTO tools (tool_type, name, endpoint, xaddr, location, username, password, enabled, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            body.tool_type, body.name, body.endpoint, body.xaddr,
            body.location, body.username, body.password,
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
        "UPDATE tools SET tool_type=?1, name=?2, endpoint=?3, xaddr=?4, location=?5, username=?6, password=?7, enabled=?8 WHERE id=?9",
        rusqlite::params![
            body.tool_type, body.name, body.endpoint, body.xaddr,
            body.location, body.username, body.password,
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

    let ports = vec![80u16, 8000, 8080];
    let semaphore = Arc::new(tokio::sync::Semaphore::new(50));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
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
        let mut open_addrs = Vec::new();
        for h in tcp_handles {
            if let Ok(Some(pair)) = h.await {
                open_addrs.push(pair);
            }
        }

        log::info!("TCP 端口扫描完成，{} 个开放端口待验证", open_addrs.len());

        // 第2步：SOAP 验证（构造 ONVIF GetDeviceInformation 请求确认设备身份）
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
        for (ip, port) in open_addrs {
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

                        // 只要响应含 SOAP Envelope（无论 200/400/401/500），说明是 ONVIF 服务
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

// ===================== 抓图 =====================

/// 从 ONVIF 摄像头抓取真实图片，返回字节数组
pub async fn fetch_snapshot(tool: &Tool) -> Result<Vec<u8>, String> {
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
    let profile = profiles.first()
        .ok_or_else(|| "摄像头没有可用 Media Profile".to_string())?;

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
