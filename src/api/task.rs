use actix_web::{get, HttpResponse, Result};
use crate::{
    db::db_path,
    models::{ApiResponse, TaskLog},
};
use rusqlite::Connection;

#[get("/api/tasks")]
pub async fn list_tasks() -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    let mut stmt = conn.prepare("SELECT * FROM task_logs ORDER BY started_at DESC LIMIT 100")
        .map_err(|e| ApiError(e.to_string()))?;

    let logs = stmt.query_map([], |row| {
        Ok(TaskLog {
            id: row.get("id")?,
            scenario_id: row.get("scenario_id")?,
            scenario_name: row.get("scenario_name")?,
            status: row.get("status")?,
            started_at: row.get("started_at")?,
            completed_at: row.get("completed_at")?,
            result: row.get("result")?,
            error_msg: row.get("error_msg")?,
        })
    }).map_err(|e| ApiError(e.to_string()))?
    .collect::<Result<Vec<_>, _>>().map_err(|e| ApiError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse::ok(logs)))
}

use crate::api::ApiError;
