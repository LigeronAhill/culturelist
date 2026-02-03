use anyhow::Result;
use config::Config;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

pub async fn get_pool(config: &Config) -> Result<Pool<Postgres>> {
    let db_url = config.get_string("database.url")?;
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&db_url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}
