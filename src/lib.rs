use anyhow::Result;
use config::Config;
use sqlx::{Pool, Postgres};

use crate::{services::UsersService, storage::UsersStorage};

pub mod configuration;
pub mod logger;
pub mod models;
mod services;
mod storage;

pub async fn build(config: &Config) -> Result<App> {
    tracing::info!("Building application");
    let pool = storage::get_pool(config).await?;
    Ok(App { pool })
}

pub struct App {
    pool: Pool<Postgres>,
}

impl App {
    pub async fn run(&self) -> Result<()> {
        let users_storage = UsersStorage::new(self.pool.clone());
        let _users_service = UsersService::new(users_storage);
        Ok(())
    }
}
