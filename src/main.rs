use std::{env, net::SocketAddr};

use tokio::net::TcpListener;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

use clearlist_api::{AppState, DatabaseConn, create_app};

// TODO: add anyhow

// Server Main
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_level(true)
        .with_target(false)
        .init();

    // Verify all environment variables exist
    let mut missing_env = false;
    if env::var("SRV_PORT").is_err() {
        eprintln!("SRV_PORT not found in environment variables");
        missing_env = true;
    }
    if env::var("DATABASE_URL").is_err() {
        eprintln!("DATABASE_URL not found in environment variables");
        missing_env = true;
    }
    // if env::var("COOKIE_KEY").is_err() {
    //     eprintln!("COOKIE_KEY not found in environment variables");
    //     missing_env = true;
    // }
    if missing_env {
        // If missing env vars, exit process.
        std::process::exit(1);
    }

    debug!("Getting environment variables");
    let srv_port = env::var("SRV_PORT").unwrap();

    // Setup Database Connection Pool
    debug!("Setting up database connection");
    let db_conn = {
        let conn = DatabaseConn::connect_env().await.unwrap();
        if !conn.is_active().await {
            eprintln!("database connection is not active");

            std::process::exit(1)
        }

        conn
    };

    // Setup app state
    let app_state = AppState::init(db_conn);

    // Init app
    let app = create_app(app_state);

    let listener = TcpListener::bind(&format!("0.0.0.0:{}", srv_port))
        .await
        .unwrap();

    info!("Starting server on port {}", srv_port);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap_or_else(|e| {
        eprintln!("unable to start server: {}", e);

        std::process::exit(1)
    });
}
