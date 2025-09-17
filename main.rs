mod db;
mod error;
mod macros;
mod response;
mod util;

mod data;
mod models;
mod routes;

use std::{env, fs, net::SocketAddr};

use axum::{Router, extract::FromRef, routing::get};
use axum_jwt_auth::JwtDecoderState;
use dotenvy::dotenv;
use jsonwebtoken::{DecodingKey, EncodingKey};
use tokio::net::TcpListener;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use crate::{
    models::jwt::Claim,
    routes::{create_api_router, health_check_handler},
    util::auth::create_decoder,
};

use db::DatabaseConn;

#[derive(Clone, FromRef)]
pub struct AppState {
    decoder: JwtDecoderState<Claim>,
    db_conn: DatabaseConn,
    decode_key: DecodingKey,
    encode_key: EncodingKey,
}

const ENV_VAR_ERR: &str = "Unable to load environment variables from `.env`";

// Server Main
#[tokio::main]
async fn main() {
    // Load .env
    dotenv().unwrap_or_else(|_| {
        eprint!("{}", ENV_VAR_ERR);

        log_error_and_exit!(ENV_VAR_ERR)
    });

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_level(true)
        .with_target(false)
        .init();

    debug!("Getting environment variables");
    let srv_port = env::var("SRV_PORT")
        .unwrap_or_else(|_| log_error_and_exit!("SRV_PORT not set"))
        .parse::<u16>()
        .unwrap_or_else(|_| log_error_and_exit!("SRV_PORT not a number"));

    let (host, port, db_name, user, pass) = get_db_env();

    let pubkey_loc =
        env::var("PUBKEY_LOC").unwrap_or_else(|_| log_error_and_exit!("PUBKEY_LOC must be set"));
    let privkey_loc =
        env::var("PRIVKEY_LOC").unwrap_or_else(|_| log_error_and_exit!("PRIVKEY_LOC must be set"));

    // Setup Database Connection Pool
    debug!("Setting up database connection");
    let db_conn = match DatabaseConn::connect(host, port, db_name, user, pass) {
        Ok(c) => {
            if !c.is_active().await {
                log_error_and_exit!("database is not active after connection");
            }

            c
        }
        Err(e) => {
            log_error_and_exit!("unable to connect to database: {:?}", e);
        }
    };

    // Get encoding and decoding keys
    debug!("Getting encoding and decoding key");
    let encode_key = match fs::read(privkey_loc) {
        Ok(k) => EncodingKey::from_ed_der(&k),
        Err(e) => {
            log_error_and_exit!("unable to read privkey: {}", e.to_string());
        }
    };

    let decode_key = match fs::read(pubkey_loc) {
        Ok(k) => DecodingKey::from_ed_der(&k),
        Err(e) => {
            log_error_and_exit!("unable to read pubkey: {}", e.to_string());
        }
    };

    // Get decoder
    debug!("Setting up token decoder");
    let decoder = match create_decoder(&decode_key) {
        Ok(d) => d,
        Err(e) => {
            log_error_and_exit!("unable to create decoder: {}", e.to_string());
        }
    };

    // Setup app state
    let app_state = AppState {
        decoder,
        db_conn,
        decode_key,
        encode_key,
    };

    // Get server routes
    let router = Router::new()
        .route("/health", get(health_check_handler))
        .nest("/api", create_api_router());

    debug!("Binding listener to port {srv_port}");
    let url = format!("0.0.0.0:{srv_port}");
    let listener = TcpListener::bind(&url).await.unwrap();

    info!("Starting server at {}", url);
    axum::serve(
        listener,
        router
            .with_state(app_state)
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap_or_else(|e| log_error_and_exit!("unable to start server: {}", e.to_string()));
}

fn get_db_env() -> (String, u16, String, String, String) {
    let mut present = true;
    let mut valid = true;
    let mut missing_vars: Vec<&str> = Vec::new();

    let host = env::var("DB_HOST").unwrap_or_else(|_| {
        present = false;
        missing_vars.push("DB_HOST");

        String::new()
    });

    let port_str = env::var("DB_PORT").unwrap_or_else(|_| {
        present = false;
        missing_vars.push("DB_PORT");

        String::new()
    });
    let port = port_str.parse::<u16>().unwrap_or_else(|_| {
        if !port_str.is_empty() {
            valid = false;
        }

        u16::MAX
    });
    let db_name = env::var("DB_NAME").unwrap_or_else(|_| {
        present = false;
        missing_vars.push("DB_NAME");

        String::new()
    });
    let user = env::var("DB_USER").unwrap_or_else(|_| {
        present = false;
        missing_vars.push("DB_USER");

        String::new()
    });
    let pass = env::var("DB_PASS").unwrap_or_else(|_| {
        present = false;
        missing_vars.push("DB_PASS");

        String::new()
    });

    if !present {
        error!("{}", format!("{} are not set", missing_vars.join(", ")));
    }
    if !valid {
        error!("DB_PORT is not a number");
    }
    if !(present && valid) {
        std::process::exit(1);
    }

    (host, port, db_name, user, pass)
}
