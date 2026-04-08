use std::{env, net::SocketAddr};

use axum::http::{HeaderValue, Method, header};
use sqlx::{Connection, PgConnection};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

use clearlist_api::{AppState, DatabaseConn, create_app, run_migration};

// TODO: add anyhow

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_level(true)
        .with_target(true)
        .init();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        if args.len() > 2 {
            eprintln!("{} expected 0-1 argument: 'function'", args[0]);

            std::process::exit(1);
        }

        match args[1].as_ref() {
            "migrate" => {
                if env::var("MIGRATION_URL").is_err() {
                    eprintln!("MIGRATION_URL not found in environment variables");

                    std::process::exit(1);
                }

                let url = env::var("MIGRATION_URL").unwrap();
                let mut conn = PgConnection::connect(&url).await.unwrap();

                // TODO: turn this into a function that can be reused in testing
                if let Err(e) = run_migration(&mut conn).await {
                    eprintln!("error occured running migration: {}", e);

                    std::process::exit(1);
                }

                println!("migration ran successfully");
            }
            _ => {
                eprintln!("unknown option found: '{}'", args[1]);

                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

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

    // Setup route logging
    let trace_layer = TraceLayer::new_for_http()
        .on_request(())
        .on_body_chunk(())
        .on_eos(())
        .on_failure(());

    let origins: Vec<HeaderValue> = env::var("ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|url| url.parse().unwrap())
        .collect();
    let headers = [
        header::CONTENT_TYPE,
        // header::AUTHORIZATION,
    ];

    // Setup CORS
    let cors = CorsLayer::new()
        .allow_credentials(true)
        .allow_origin(origins)
        .allow_headers(headers)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]);

    // Init app
    let app = create_app(app_state).layer(ServiceBuilder::new().layer(trace_layer).layer(cors));

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
