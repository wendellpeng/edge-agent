use crate::{
    db::db_path,
    models::{NodeResult, Scenario, TaskResult, Tool},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Local;
use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};

pub struct TaskEngine {
    scheduler: Arc<Mutex<JobScheduler>>,
    first_reload: AtomicBool,
}

impl TaskEngine {
    pub async fn new() -> Result<Self, String> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| format!("创建调度器失败: {}", e))?;
        Ok(TaskEngine {
            scheduler: Arc::new(Mutex::new(scheduler)),
            first_reload: AtomicBool::new(true),
        })
    }

    pub async fn start(&self) -> Result<(), String> {
        {
            let sched = self.scheduler.lock().await;
            sched.start().await.map_err(|e| e.to_string())?;
        }
        self.reload_jobs().await?;
        log::info!("任务引擎已启动");
        Ok(())
    }

    pub async fn reload_jobs(&self) -> Result<(), String> {
        let conn = Connection::open(db_path()).map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT * FROM scenarios WHERE enabled = 1")
            .map_err(|e| e.to_string())?;

        let scenarios: Vec<Scenario> = stmt.query_map([], |row| {
            let pipeline_str: String = row.get("pipeline")?;
            Ok(Scenario {
                id: row.get("id")?,
                name: row.get("name")?,
                enabled: row.get::<_, i32>("enabled")? == 1,
                pipeline: serde_json::from_str(&pipeline_str).unwrap_or_default(),
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        }).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

        let mut sched = self.scheduler.lock().await;

        // 只有非首次 reload 时才 shutdown + recreate，避免第一次启动时的 Closed 错误
        if !self.first_reload.swap(false, Ordering::SeqCst) {
            sched.shutdown().await.ok();
            *sched = JobScheduler::new().await.map_err(|e| e.to_string())?;
            sched.start().await.map_err(|e| e.to_string())?;
            log::info!("调度器已重置，开始加载 {} 个场景...", scenarios.len());
        } else {
            log::info!("首次加载 {} 个场景...", scenarios.len());
        }

        for scenario in scenarios {
            let trigger = scenario.pipeline.iter().find(|n| n.node_type == "trigger");
            let Some(trigger) = trigger else {
                log::warn!("场景 [{}] 没有触发器节点，跳过调度", scenario.name);
                continue;
            };

            let trigger_type = trigger.config.get("trigger_type")
                .and_then(|v| v.as_str())
                .unwrap_or("manual");
            let trigger_config = trigger.config.get("config")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let scenario_clone = scenario.clone();

            match trigger_type {
                "cron" => {
                    let job = match Job::new_async(trigger_config, move |_uuid, _l| {
                        let s = scenario_clone.clone();
                        Box::pin(async move {
                            if let Err(e) = execute_scenario(&s).await {
                                log::error!("场景 [{}] 执行失败: {}", s.name, e);
                            }
                        })
                    }) {
                        Ok(j) => j,
                        Err(e) => {
                            log::error!("场景 [{}] cron 表达式 '{}' 解析失败: {}", scenario.name, trigger_config, e);
                            continue;
                        }
                    };
                    sched.add(job).await.map_err(|e| format!("添加任务失败: {}", e))?;
                    log::info!("场景 [{}] 已加入 cron 调度: {}", scenario.name, trigger_config);
                }
                "interval" => {
                    let secs = trigger_config.parse::<u64>().unwrap_or(60);
                    let duration = Duration::from_secs(secs);
                    let job = Job::new_repeated_async(duration, move |_uuid, _l| {
                        let s = scenario_clone.clone();
                        Box::pin(async move {
                            if let Err(e) = execute_scenario(&s).await {
                                log::error!("场景 [{}] 执行失败: {}", s.name, e);
                            }
                        })
                    }).map_err(|e| format!("创建间隔任务失败: {}", e))?;
                    sched.add(job).await.map_err(|e| e.to_string())?;
                    log::info!("场景 [{}] 已加入 interval 调度: 每 {} 秒", scenario.name, secs);
                }
                _ => {
                    log::warn!("场景 [{}] 触发类型 '{}' 不支持自动调度", scenario.name, trigger_type);
                }
            }
        }

        // 添加日志清理定时任务：每天凌晨 3 点执行
        let cleanup_job = Job::new_async("0 3 * * *", move |_uuid, _l| {
            Box::pin(async move {
                match crate::api::settings::cleanup_old_logs() {
                    Ok(deleted) => {
                        if deleted > 0 {
                            log::info!("定时日志清理完成，删除 {} 条", deleted);
                        }
                    }
                    Err(e) => log::error!("定时日志清理失败: {}", e),
                }
            })
        });
        match cleanup_job {
            Ok(job) => {
                if let Err(e) = sched.add(job).await {
                    log::warn!("添加日志清理定时任务失败: {}", e);
                } else {
                    log::info!("日志清理定时任务已添加（每天凌晨 3 点）");
                }
            }
            Err(e) => log::warn!("创建日志清理定时任务失败: {}", e),
        }

        Ok(())
    }
}

async fn execute_scenario(scenario: &Scenario) -> Result<(), String> {
    log::info!("开始执行场景: {}", scenario.name);
    let task_id = insert_task_log(scenario, "running")?;

    let mut node_results = Vec::new();
    let mut last_image_path: Option<String> = None;

    for node in &scenario.pipeline {
        match node.node_type.as_str() {
            "trigger" => {
                node_results.push(NodeResult {
                    node_id: node.node_id.clone(),
                    node_type: "trigger".to_string(),
                    success: true,
                    output: None,
                    error: None,
                });
            }
            "tool" => {
                let tool_id = node.config.get("tool_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| "工具节点缺少 tool_id".to_string())?;

                let tool = load_tool(tool_id)?;
                if !tool.enabled {
                    node_results.push(NodeResult {
                        node_id: node.node_id.clone(),
                        node_type: "tool".to_string(),
                        success: false,
                        output: None,
                        error: Some("工具已禁用".to_string()),
                    });
                    continue;
                }

                match take_snapshot(&tool).await {
                    Ok(path) => {
                        log::info!("工具 [{}] 抓拍成功", tool.name);
                        last_image_path = Some(path.clone());
                        node_results.push(NodeResult {
                            node_id: node.node_id.clone(),
                            node_type: "tool".to_string(),
                            success: true,
                            output: Some(path),
                            error: None,
                        });
                    }
                    Err(e) => {
                        log::error!("工具 [{}] 抓拍失败: {}", tool.name, e);
                        node_results.push(NodeResult {
                            node_id: node.node_id.clone(),
                            node_type: "tool".to_string(),
                            success: false,
                            output: None,
                            error: Some(e.clone()),
                        });
                    }
                }
            }
            "http_push" => {
                let url = node.config.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let method = node.config.get("method").and_then(|v| v.as_str()).unwrap_or("POST");
                let timeout_secs = node.config.get("timeout_secs").and_then(|v| v.as_u64()).unwrap_or(30);
                let retry_count = node.config.get("retry_count").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(3);
                let image_param_name = node.config.get("image_param_name").and_then(|v| v.as_str()).unwrap_or("image");
                let body_params_raw = node.config.get("body_params").and_then(|v| v.as_str()).unwrap_or("");

                let mut headers_map = std::collections::HashMap::new();
                if let Some(h) = node.config.get("headers") {
                    if let Ok(hm) = serde_json::from_value::<std::collections::HashMap<String, String>>(h.clone()) {
                        headers_map = hm;
                    }
                }

                let mut payload_success = false;
                let mut push_err = None;

                if let Some(ref path) = last_image_path {
                    match std::fs::read(path) {
                        Ok(bytes) => {
                            let b64 = STANDARD.encode(&bytes);

                            // 构造 payload：先解析 body_params JSON，再注入图片
                            let mut payload = serde_json::json!({});
                            if !body_params_raw.is_empty() {
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body_params_raw) {
                                    if let Some(obj) = parsed.as_object() {
                                        for (k, v) in obj {
                                            payload[k] = v.clone();
                                        }
                                    }
                                }
                            }
                            payload[image_param_name] = serde_json::json!(b64);
                            payload["scenario_name"] = serde_json::json!(&scenario.name);
                            payload["timestamp"] = serde_json::json!(chrono::Local::now().to_rfc3339());

                            let client = reqwest::Client::new();
                            let timeout = std::time::Duration::from_secs(timeout_secs);
                            let retry = retry_count.max(1);
                            let mut last_err = String::new();

                            let mut req = client
                                .request(
                                    method.parse().map_err(|_| "无效的 HTTP 方法".to_string())?,
                                    url,
                                )
                                .json(&payload)
                                .timeout(timeout);

                            for (k, v) in &headers_map {
                                req = req.header(k, v);
                            }

                            for attempt in 1..=retry {
                                match req.try_clone().unwrap().send().await {
                                    Ok(resp) => {
                                        if resp.status().is_success() {
                                            let body = resp.text().await.unwrap_or_default();
                                            log::info!("推送成功: {}", body);
                                            payload_success = true;
                                            break;
                                        } else {
                                            let status = resp.status();
                                            let body = resp.text().await.unwrap_or_default();
                                            last_err = format!("HTTP {}: {}", status, body);
                                            log::warn!("推送失败 (尝试 {}/{}): {}", attempt, retry, last_err);
                                        }
                                    }
                                    Err(e) => {
                                        last_err = format!("请求异常: {}", e);
                                        log::warn!("推送失败 (尝试 {}/{}): {}", attempt, retry, last_err);
                                    }
                                }
                                if attempt < retry {
                                    tokio::time::sleep(std::time::Duration::from_secs(2_u64.pow(attempt - 1))).await;
                                }
                            }

                            if !payload_success {
                                push_err = Some(format!("推送最终失败 (已重试 {} 次): {}", retry, last_err));
                            }
                        }
                        Err(e) => {
                            push_err = Some(format!("读取图片失败: {}", e));
                        }
                    }
                } else {
                    push_err = Some("没有可推送的图片".to_string());
                }

                node_results.push(NodeResult {
                    node_id: node.node_id.clone(),
                    node_type: "http_push".to_string(),
                    success: payload_success,
                    output: None,
                    error: push_err,
                });
            }
            _ => {
                log::warn!("未知节点类型: {}", node.node_type);
                node_results.push(NodeResult {
                    node_id: node.node_id.clone(),
                    node_type: node.node_type.clone(),
                    success: false,
                    output: None,
                    error: Some(format!("未知节点类型: {}", node.node_type)),
                });
            }
        }
    }

    let result_json = serde_json::to_string(&TaskResult {
        scenario_id: scenario.id,
        node_results: node_results.clone(),
    }).unwrap_or_default();

    let all_success = node_results.iter().all(|r| r.success);
    let status = if all_success { "success" } else { "failed" };
    let error_msg = if all_success { None } else {
        Some(node_results.iter()
            .filter(|r| !r.success)
            .map(|r| format!("{}: {}", r.node_id, r.error.as_deref().unwrap_or("未知错误")))
            .collect::<Vec<_>>()
            .join("; "))
    };

    update_task_log(task_id, status, Some(result_json), error_msg)?;
    log::info!("场景 [{}] 执行完成: {}", scenario.name, status);
    Ok(())
}

fn load_tool(id: i64) -> Result<Tool, String> {
    let conn = Connection::open(db_path()).map_err(|e| e.to_string())?;
    let tool: Tool = conn.query_row(
        "SELECT * FROM tools WHERE id = ?1 AND enabled = 1",
        [id],
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
    ).map_err(|e| format!("加载工具失败: {}", e))?;
    Ok(tool)
}

async fn take_snapshot(tool: &Tool) -> Result<String, String> {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("com.edgeagent.app/snapshots");
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("创建缓存目录失败: {}", e))?;

    let filename = format!("tool{}_{}.jpg", tool.id, Local::now().format("%Y%m%d_%H%M%S"));
    let path = cache_dir.join(&filename);

    let bytes = crate::api::tool::fetch_snapshot(tool).await?;
    std::fs::write(&path, bytes).map_err(|e| format!("保存快照失败: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}

fn insert_task_log(scenario: &Scenario, status: &str) -> Result<i64, String> {
    let conn = Connection::open(db_path()).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO task_logs (scenario_id, scenario_name, status, started_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![scenario.id, &scenario.name, status, Local::now().to_rfc3339()],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

fn update_task_log(id: i64, status: &str, result: Option<String>, error_msg: Option<String>) -> Result<(), String> {
    let conn = Connection::open(db_path()).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE task_logs SET status = ?1, completed_at = ?2, result = ?3, error_msg = ?4 WHERE id = ?5",
        rusqlite::params![status, Local::now().to_rfc3339(), result, error_msg, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn run_scenario_manual(scenario_id: i64) -> Result<(), String> {
    let conn = Connection::open(db_path()).map_err(|e| e.to_string())?;
    let scenario: Scenario = conn.query_row(
        "SELECT * FROM scenarios WHERE id = ?1",
        [scenario_id],
        |row| {
            let pipeline_str: String = row.get("pipeline")?;
            Ok(Scenario {
                id: row.get("id")?,
                name: row.get("name")?,
                enabled: row.get::<_, i32>("enabled")? == 1,
                pipeline: serde_json::from_str(&pipeline_str).unwrap_or_default(),
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    ).map_err(|e| format!("查询场景失败: {}", e))?;

    execute_scenario(&scenario).await
}
