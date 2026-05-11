use std::path::Path;
use anyhow::anyhow;
use boss_mcp::{browser, config::load_or_create, mcp_server::RecruitmentServer, ServiceExt};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = load_or_create(Path::new("config.yaml"))
        .map_err(|e| anyhow!(format!("配置加载异常:{}", e)))?;

    browser::init(config.clone()).map_err(|e| anyhow!(format!("浏览器加载异常:{}", e)))?;

    let server = RecruitmentServer::new();

    match config.mcp.transport {
        boss_mcp::config::TransportType::Stdio => {
            println!("Starting MCP Server in stdio mode...");
            let transport = rmcp::transport::stdio();
            server.serve(transport).await?;
        }
        boss_mcp::config::TransportType::StreamableHttp => {
            use rmcp::transport::streamable_http_server::{
                StreamableHttpService,
                StreamableHttpServerConfig,
                session::local::LocalSessionManager,
            };
            use std::sync::Arc;
            use axum::Router;

            println!("Starting MCP Server in streamable_http mode...");
            let session_manager = Arc::new(LocalSessionManager::default());
            let service = StreamableHttpService::new(
                move || Ok(RecruitmentServer::new()),
                session_manager,
                StreamableHttpServerConfig::default(),
            );

            let qr_dir = std::path::PathBuf::from(config.qr_output_path());
            if let Some(parent) = qr_dir.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            let app = Router::new()
                .route("/", axum::routing::get(|| async { "boss_mcp running" }))
                .nest_service("/mcp", service)
                .nest_service("/static/qr", ServeDir::new(&qr_dir));

            let addr = format!("{}:{}", config.mcp.http_host, config.mcp.http_port);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            println!("MCP Server listening on http://{}", addr);
            println!("QR files served at {}/static/qr/", config.mcp.public_base_url());
            axum::serve(listener, app).await?;
        }
    };

    Ok(())
}
