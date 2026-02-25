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

    debug!("Getting environment variables");
    let srv_port = match env::var("SRV_PORT") {
        Ok(port) => port,
        Err(_) => {
            eprintln!("SRV_PORT not found in environment variables");

            std::process::exit(1)
        }
    };

    // Setup Database Connection Pool
    debug!("Setting up database connection");
    let db_conn = match DatabaseConn::connect_env() {
        Ok(conn) => {
            if !conn.is_active().await {
                eprintln!("database connection is not active");

                std::process::exit(1)
            }

            conn
        }
        Err(e) => {
            eprintln!("unable to connect to database: {:?}", e);

            std::process::exit(1)
        }
    };

    // Setup app state
    let app_state = AppState { db_conn };

    // Init app
    let app = create_app(app_state);

    debug!("Binding listener to port {srv_port}");
    let url = format!("0.0.0.0:{srv_port}");
    let listener = TcpListener::bind(&url).await.unwrap();

    info!("Starting server at {}", url);
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
