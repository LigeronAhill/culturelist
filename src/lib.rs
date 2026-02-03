use anyhow::Result;
use config::Config;
use sqlx::{Pool, Postgres};

pub mod configuration;
pub mod logger;
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
        _ = self.pool;
        Ok(())
    }
}
