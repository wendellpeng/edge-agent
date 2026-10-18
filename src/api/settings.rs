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
    let deleted = conn.execute(
        "DELETE FROM task_logs WHERE started_at < datetime('now', ?1)",
        [&cutoff],
    ).map_err(|e| e.to_string())?;

    if deleted > 0 {
        log::info!("日志清理完成：删除了 {} 条超过 {} 天的历史日志", deleted, days);
    }

    Ok(deleted)
}
