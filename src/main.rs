mod api;
mod credential_store;
mod db;
mod http_pusher;
mod models;
mod task_engine;

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    if let Err(e) = db::init_db() {
        log::error!("数据库初始化失败: {}", e);
    }

    let task_engine = Arc::new(Mutex::new(
        task_engine::TaskEngine::new().await.expect("任务引擎初始化失败")
    ));

    {
        let engine = task_engine.clone();
        let e = engine.lock().await;
        if let Err(err) = e.start().await {
            log::error!("任务引擎启动失败: {}", err);
        }
    }

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    log::info!("视觉巡检智能体 启动于 http://{}", bind_addr);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(task_engine.clone()))
            .wrap(middleware::Logger::default())
            .wrap(cors)
            // 工具 API
            .service(api::tool::list_tools)
            .service(api::tool::add_tool)
            .service(api::tool::update_tool)
            .service(api::tool::delete_tool)
            .service(api::tool::discover_tools)
            .service(api::tool::test_snapshot)
            .service(api::tool::list_channels)
            .service(api::tool::list_rtsp_templates)
            .service(api::tool::generate_rtsp_url)
            // 场景 API
            .service(api::scenario::list_scenarios)
            .service(api::scenario::add_scenario)
            .service(api::scenario::update_scenario)
            .service(api::scenario::delete_scenario)
            .service(api::scenario::run_scenario)
            // 任务 API
            .service(api::task::list_tasks)
            // 静态文件 / SPA fallback
            .route("/", web::get().to(serve_embedded))
            .route("/{filename:.*}", web::get().to(serve_embedded))
    })
    .bind(&bind_addr)?
    .run()
    .await
}
