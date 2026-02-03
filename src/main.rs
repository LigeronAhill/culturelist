#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = app::configuration::init()?;
    app::logger::init(&config)?;
    let application = app::build(&config).await?;
    application.run().await?;
    Ok(())
}
