#![allow(dead_code)]

use crate::models::PushConfig;
use serde_json::json;

pub async fn push_result(
    config: &PushConfig,
    scenario_name: &str,
    camera_name: &str,
    llm_result: &str,
    image_base64: Option<&str>,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let timeout = std::time::Duration::from_secs(config.timeout_secs.unwrap_or(30));

    let mut payload = json!({
        "scenario_name": scenario_name,
        "camera_name": camera_name,
        "analysis_result": llm_result,
        "timestamp": chrono::Local::now().to_rfc3339(),
    });

    if let Some(img) = image_base64 {
        payload["image_base64"] = json!(img);
    }

    let mut req = client
        .request(
            config.method.parse().map_err(|_| "无效的 HTTP 方法".to_string())?,
            &config.url,
        )
        .json(&payload)
        .timeout(timeout);

    if let Some(ref headers) = config.headers {
        for (k, v) in headers {
            req = req.header(k, v);
        }
    }

    let retry = config.retry_count.unwrap_or(1).max(1);
    let mut last_err = String::new();

    for attempt in 1..=retry {
        match req.try_clone().unwrap().send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    log::info!("推送成功 [{}]: {}", scenario_name, body);
                    return Ok(body);
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

    Err(format!("推送最终失败 (已重试 {} 次): {}", retry, last_err))
}
