pub mod credential;
pub mod scenario;
pub mod task;
pub mod tool;

use actix_web::{HttpResponse, ResponseError};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub struct ApiError(pub String);

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": self.0
        }))
    }
}
