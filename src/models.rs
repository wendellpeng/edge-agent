use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CameraDiscovery {
    pub endpoint: String,
    pub xaddr: Option<String>,
    pub name: Option<String>,
    pub scopes: Vec<String>,
    pub types: Vec<String>,
    #[serde(default)]
    pub source: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tool {
    #[serde(default)]
    pub id: i64,
    pub tool_type: String,
    pub name: String,
    pub endpoint: String,
    pub xaddr: Option<String>,
    pub location: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(default = "default_channel")]
    pub channel: i32,
    #[serde(default)]
    pub brand: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub created_at: String,
}

fn default_channel() -> i32 { 1 }

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct CredentialMeta {
    pub id: String,
    pub service: String,
    pub username: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct CredentialInput {
    pub id: String,
    pub service: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipelineNode {
    pub node_id: String,
    pub node_type: String,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Scenario {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub pipeline: Vec<PipelineNode>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct PushConfig {
    pub url: String,
    pub method: String,
    pub headers: Option<HashMap<String, String>>,
    pub timeout_secs: Option<u64>,
    pub retry_count: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskLog {
    pub id: i64,
    pub scenario_id: i64,
    pub scenario_name: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub result: Option<String>,
    pub error_msg: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskResult {
    pub scenario_id: i64,
    pub node_results: Vec<NodeResult>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeResult {
    pub node_id: String,
    pub node_type: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        ApiResponse { success: true, data: Some(data), error: None }
    }
    #[allow(dead_code)]
    pub fn err(msg: String) -> Self {
        ApiResponse { success: false, data: None, error: Some(msg) }
    }
}
