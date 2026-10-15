use actix_web::{delete, get, post, put, web, HttpResponse, Result};
use crate::{
    db::db_path,
    models::{ApiResponse, Scenario},
    task_engine,
};
use chrono::Local;
use rusqlite::Connection;

#[get("/api/scenarios")]
pub async fn list_scenarios() -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    let mut stmt = conn.prepare("SELECT * FROM scenarios ORDER BY updated_at DESC")
        .map_err(|e| ApiError(e.to_string()))?;

    let scenarios = stmt.query_map([], |row| {
        let pipeline_str: String = row.get("pipeline")?;
        Ok(Scenario {
            id: row.get("id")?,
            name: row.get("name")?,
            enabled: row.get::<_, i32>("enabled")? == 1,
            pipeline: serde_json::from_str(&pipeline_str).unwrap_or_default(),
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }).map_err(|e| ApiError(e.to_string()))?
    .collect::<Result<Vec<_>, _>>().map_err(|e| ApiError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse::ok(scenarios)))
}

#[post("/api/scenarios")]
pub async fn add_scenario(body: web::Json<Scenario>) -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    let pipeline = serde_json::to_string(&body.pipeline).unwrap_or_default();
    let now = Local::now().to_rfc3339();

    conn.execute(
        "INSERT INTO scenarios (name, enabled, pipeline, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            &body.name, if body.enabled { 1 } else { 0 }, pipeline, &now, &now,
        ],
    ).map_err(|e| ApiError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(conn.last_insert_rowid())))
}

#[put("/api/scenarios/{id}")]
pub async fn update_scenario(path: web::Path<i64>, body: web::Json<Scenario>) -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    let pipeline = serde_json::to_string(&body.pipeline).unwrap_or_default();

    conn.execute(
        "UPDATE scenarios SET name=?1, enabled=?2, pipeline=?3, updated_at=?4 WHERE id=?5",
        rusqlite::params![
            &body.name, if body.enabled { 1 } else { 0 }, pipeline,
            Local::now().to_rfc3339(), *path,
        ],
    ).map_err(|e| ApiError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok("updated")))
}

#[delete("/api/scenarios/{id}")]
pub async fn delete_scenario(path: web::Path<i64>) -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    conn.execute("DELETE FROM scenarios WHERE id = ?1", [*path])
        .map_err(|e| ApiError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok("deleted")))
}

#[post("/api/scenarios/{id}/run")]
pub async fn run_scenario(path: web::Path<i64>) -> Result<HttpResponse> {
    task_engine::run_scenario_manual(*path).await
        .map_err(|e| ApiError(e))?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok("triggered")))
}

use crate::api::ApiError;
