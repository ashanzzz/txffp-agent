use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use txffp_server::api::create_router;
use txffp_server::auth::AuthManager;
use txffp_server::browser::SteelBrowserDriver;
use txffp_server::config::AppConfig;
use txffp_server::credential::FileAndEnvCredentialProvider;
use txffp_server::db::init_db;
use txffp_server::human_action::HumanActionManager;
use txffp_server::service::TxffpService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "txffp_server=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Txffp Agent Server...");

    // 2. Load configuration
    let config = AppConfig::load();
    tracing::info!(
        "Config loaded: host={}, port={}, auth_mode={:?}, data_dir={}",
        config.host,
        config.port,
        config.auth_mode,
        config.data_dir.display()
    );

    // 3. Initialize SQLite
    let pool = init_db(&config.database_url).await?;
    tracing::info!(
        "Database initialized successfully at {}",
        config.database_url
    );

    // 4. Initialize core components
    let credential_provider = Arc::new(FileAndEnvCredentialProvider::new(
        config.secrets_path.clone(),
    ));
    let auth_manager = Arc::new(AuthManager::new());
    let human_action_manager = Arc::new(HumanActionManager::new(pool.clone()));
    let browser_driver = Arc::new(SteelBrowserDriver::new(&config.steel_base_url));

    let service = Arc::new(TxffpService::new(
        config.clone(),
        pool,
        credential_provider,
        auth_manager,
        human_action_manager,
        browser_driver,
    ));

    // 5. Construct Axum app router
    let app = create_router(service)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    // 6. Bind and serve
    let addr = format!("{}:{}", config.host, config.port).parse::<SocketAddr>()?;
    tracing::info!("Txffp Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
