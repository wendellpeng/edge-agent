use actix_web::{get, put, web, HttpResponse, Result};
use chrono::Local;
use rusqlite::Connection;
use crate::{
    db::db_path,
    models::ApiResponse,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogRetentionSetting {
    pub days: i32,
    pub label: String,
}

#[get("/api/settings/log-retention")]
pub async fn get_log_retention() -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| crate::api::ApiError(e.to_string()))?;
    let days: i32 = conn.query_row(
        "SELECT value FROM settings WHERE key = 'log_retention_days'",
        [], |row| {
            let s: String = row.get(0)?;
            Ok(s.parse::<i32>().unwrap_or(90))
        },
    ).unwrap_or(90);

    let label = match days {
        30 => "1个月",
        90 => "3个月",
        180 => "半年",
        _ => "自定义",
    };

    Ok(HttpResponse::Ok().json(ApiResponse::ok(LogRetentionSetting { days, label: label.to_string() })))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateLogRetentionRequest {
    pub days: i32,
}

#[put("/api/settings/log-retention")]
pub async fn update_log_retention(body: web::Json<UpdateLogRetentionRequest>) -> Result<HttpResponse> {
    let days = body.days;
    if ![30, 90, 180].contains(&days) {
        return Ok(HttpResponse::Ok().json(ApiResponse::<()>::err("保留时间必须是 30/90/180 天".to_string())));
    }

    let conn = Connection::open(db_path()).map_err(|e| crate::api::ApiError(e.to_string()))?;
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES ('log_retention_days', ?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        [days.to_string(), Local::now().to_rfc3339()],
    ).map_err(|e| crate::api::ApiError(e.to_string()))?;

    let label = match days {
        30 => "1个月",
        90 => "3个月",
        180 => "半年",
        _ => "自定义",
    };

    Ok(HttpResponse::Ok().json(ApiResponse::ok(LogRetentionSetting { days, label: label.to_string() })))
}

/// 执行日志清理，返回删除的条数
/// 清理前会先删除 task_logs.result 中关联的照片文件
pub fn cleanup_old_logs() -> Result<usize, String> {
    let conn = Connection::open(db_path()).map_err(|e| e.to_string())?;

    let days: i32 = conn.query_row(
        "SELECT value FROM settings WHERE key = 'log_retention_days'",
        [], |row| {
            let s: String = row.get(0)?;
            Ok(s.parse::<i32>().unwrap_or(90))
        },
    ).unwrap_or(90);

    let cutoff = format!("{} days", -days);

    // Step 1: 找出要删除的日志，提取关联的照片路径并删除
    let mut stmt = conn.prepare(
        "SELECT result FROM task_logs WHERE started_at < datetime('now', ?1) AND result IS NOT NULL"
    ).map_err(|e| e.to_string())?;

    let results = stmt.query_map([&cutoff], |row| {
        let result_str: String = row.get(0)?;
        Ok(result_str)
    }).map_err(|e| e.to_string())?;

    let mut files_deleted = 0;
    let mut files_failed = 0;

    for result_str in results.flatten() {
        if let Ok(task_result) = serde_json::from_str::<serde_json::Value>(&result_str) {
            if let Some(node_results) = task_result.get("node_results").and_then(|v| v.as_array()) {
                for node in node_results {
                    if let Some(output) = node.get("output").and_then(|v| v.as_str()) {
                        if output.ends_with(".jpg") || output.ends_with(".jpeg") || output.ends_with(".png") {
                            match std::fs::remove_file(output) {
                                Ok(_) => files_deleted += 1,
                                Err(e) => {
                                    // 文件可能已被手动删除，只记录 debug 日志
                                    log::debug!("删除照片失败 {}: {}", output, e);
                                    files_failed += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 2: 删除数据库记录
    let deleted = conn.execute(
        "DELETE FROM task_logs WHERE started_at < datetime('now', ?1)",
        [&cutoff],
    ).map_err(|e| e.to_string())?;

    if deleted > 0 || files_deleted > 0 {
        log::info!(
            "日志清理完成：删除 {} 条日志，关联照片 {} 张（失败 {} 张），保留 {} 天",
            deleted, files_deleted, files_failed, days
        );
    }

    Ok(deleted)
}
