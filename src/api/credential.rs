use actix_web::{delete, get, post, web, HttpResponse, Result};
use crate::{
    credential_store,
    db::db_path,
    models::{ApiResponse, CredentialInput, CredentialMeta},
};
use chrono::Local;
use rusqlite::Connection;

#[get("/api/credentials")]
pub async fn list_credentials() -> Result<HttpResponse> {
    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    let mut stmt = conn.prepare("SELECT * FROM credentials ORDER BY created_at DESC")
        .map_err(|e| ApiError(e.to_string()))?;

    let creds = stmt.query_map([], |row| {
        Ok(CredentialMeta {
            id: row.get("id")?,
            service: row.get("service")?,
            username: row.get("username")?,
            created_at: row.get("created_at")?,
        })
    }).map_err(|e| ApiError(e.to_string()))?
    .collect::<Result<Vec<_>, _>>().map_err(|e| ApiError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse::ok(creds)))
}

#[post("/api/credentials")]
pub async fn save_credential(body: web::Json<CredentialInput>) -> Result<HttpResponse> {
    credential_store::save_credential(&body.id, &body.service, &body.username, &body.password)
        .map_err(|e| ApiError(e))?;

    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    conn.execute(
        "INSERT OR REPLACE INTO credentials (id, service, username, created_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![&body.id, &body.service, &body.username, Local::now().to_rfc3339()],
    ).map_err(|e| ApiError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse::ok("saved")))
}

#[delete("/api/credentials/{id}")]
pub async fn delete_credential(path: web::Path<String>, query: web::Query<std::collections::HashMap<String, String>>) -> Result<HttpResponse> {
    let username = query.get("username").cloned().unwrap_or_default();
    credential_store::delete_credential(&path, &username).map_err(|e| ApiError(e))?;

    let conn = Connection::open(db_path()).map_err(|e| ApiError(e.to_string()))?;
    conn.execute("DELETE FROM credentials WHERE id = ?1", [&*path])
        .map_err(|e| ApiError(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse::ok("deleted")))
}

use crate::api::ApiError;
