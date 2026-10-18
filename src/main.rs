#![cfg_attr(windows, windows_subsystem = "windows")]

mod api;
mod credential_store;
mod db;
mod http_pusher;
mod models;
mod strix;
mod task_engine;

#[cfg(windows)]
mod service;

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use rust_embed::RustEmbed;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct FrontendAssets;

async fn serve_embedded(path: Option<web::Path<String>>) -> actix_web::HttpResponse {
    let path = path.map(|p| p.into_inner()).unwrap_or_default();
    let asset_path = if path.is_empty() || path == "/" {
        "index.html"
    } else {
        &path
    };

    match FrontendAssets::get(asset_path) {
        Some(content) => {
            let mime = match asset_path.rsplit('.').next() {
                Some("js") => "application/javascript",
                Some("css") => "text/css",
                Some("html") => "text/html",
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("svg") => "image/svg+xml",
                Some("json") => "application/json",
                _ => "application/octet-stream",
            };
            actix_web::HttpResponse::Ok()
                .content_type(mime)
                .body(content.data.into_owned())
        }
        None => {
            match FrontendAssets::get("index.html") {
                Some(content) => actix_web::HttpResponse::Ok()
                    .content_type("text/html")
                    .body(content.data.into_owned()),
                None => actix_web::HttpResponse::NotFound().body("Not Found"),
            }
        }
    }
}

/// Initialize and return the HTTP server (does not block).
pub async fn init_agent() -> std::io::Result<actix_web::dev::Server> {
    env_logger::init();

    if let Err(e) = db::init_db() {
        log::error!("Database init failed: {}", e);
    }

    let task_engine = Arc::new(Mutex::new(
        task_engine::TaskEngine::new().await.expect("Task engine init failed")
    ));

    {
        let engine = task_engine.clone();
        let e = engine.lock().await;
        if let Err(err) = e.start().await {
            log::error!("Task engine start failed: {}", err);
        }
    }

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    log::info!("Edge Agent starting at http://{}", bind_addr);

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(task_engine.clone()))
            .wrap(middleware::Logger::default())
            .wrap(cors)
            // Tool APIs
            .service(api::tool::list_tools)
            .service(api::tool::add_tool)
            .service(api::tool::update_tool)
            .service(api::tool::delete_tool)
            .service(api::tool::discover_tools)
            .service(api::tool::test_snapshot)
            .service(api::tool::list_channels)
            .service(api::tool::list_rtsp_templates)
            .service(api::tool::generate_rtsp_url)
            .service(api::tool::probe_device)
            .service(api::tool::verify_rtsp_url_endpoint)
            // Scenario APIs
            .service(api::scenario::list_scenarios)
            .service(api::scenario::add_scenario)
            .service(api::scenario::update_scenario)
            .service(api::scenario::delete_scenario)
            .service(api::scenario::run_scenario)
            // Task APIs
            .service(api::task::list_tasks)
            // Settings APIs
            .service(api::settings::get_log_retention)
            .service(api::settings::update_log_retention)
            // Static files / SPA fallback
            .route("/", web::get().to(serve_embedded))
            .route("/{filename:.*}", web::get().to(serve_embedded))
    })
    .bind(&bind_addr)?
    .run();

    Ok(server)
}

#[cfg(not(windows))]
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let server = init_agent().await?;
    server.await
}

#[cfg(windows)]
fn main() {
    // Try to run as Windows service first.
    // If not started by SCM, service::run() returns ERROR_FAILED_SERVICE_CONTROLLER_CONNECT (1063).
    match service::run() {
        Ok(()) => return,
        Err(windows_service::Error::Winapi(e))
            if e.raw_os_error() == Some(1063) => {
            // Not started by Service Control Manager — continue with normal mode.
        }
        Err(e) => {
            eprintln!("Service error: {}", e);
            std::process::exit(1);
        }
    }

    // Normal mode: create tokio runtime and run the HTTP server.
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    if let Err(e) = rt.block_on(async {
        let server = init_agent().await?;
        server.await
    }) {
        eprintln!("Agent error: {}", e);
        std::process::exit(1);
    }
}
