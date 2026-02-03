use anyhow::Result;
use axum_session::SessionConfig;
use axum_session_sqlx::SessionPgSessionStore;
use config::Config;
use sqlx::{Pool, Postgres};

use crate::{services::UsersService, storage::UsersStorage};

pub mod configuration;
pub mod controllers;
pub mod logger;
pub mod models;
mod router;
mod services;
mod storage;

pub async fn build(config: &Config) -> Result<App> {
    tracing::info!("Building application");
    let pool = storage::get_pool(config).await?;
    let port = config.get_int("server.port").unwrap_or(3000) as u16;
    Ok(App { pool, port })
}

pub struct App {
    pool: Pool<Postgres>,
    port: u16,
}

#[derive(Clone)]
pub struct AppState {
    pub users_service: UsersService,
}

impl App {
    pub async fn run(&self) -> Result<()> {
        // sessions
        let session_config = SessionConfig::default().with_table_name("sessions_table");
        let session_store =
            SessionPgSessionStore::new(Some(self.pool.clone().into()), session_config)
                .await
                .unwrap();

        // services
        let users_storage = UsersStorage::new(self.pool.clone());
        let users_service = UsersService::new(users_storage);

        // app state
        let app_state = AppState { users_service };

        // server
        let addr = format!("0.0.0.0:{p}", p = self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let service = router::init(&format!("http://{}", addr), session_store, app_state);
        axum::serve(listener, service)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }
}

async fn shutdown_signal() {
    use tokio::signal;
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
