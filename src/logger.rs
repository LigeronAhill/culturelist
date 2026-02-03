use anyhow::Result;
use config::Config;

pub fn init(config: &Config) -> Result<()> {
    let env = config
        .get_string("app.environment")
        .unwrap_or("development".into());
    match env.as_str() {
        "production" => {
            let subscriber = tracing_subscriber::FmtSubscriber::builder()
                .with_max_level(tracing::Level::ERROR)
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .pretty()
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        _ => {
            let subscriber = tracing_subscriber::FmtSubscriber::builder()
                .with_max_level(tracing::Level::INFO)
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .pretty()
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
    }
    tracing::info!("logger initialized");
    Ok(())
}
