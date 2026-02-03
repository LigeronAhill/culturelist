use config::{Config, ConfigError};

pub fn init() -> Result<Config, ConfigError> {
    let env = std::env::var("APP_ENVIRONMENT").unwrap_or("development".into());
    let root = std::env!("CARGO_MANIFEST_DIR");
    let base_path = std::path::PathBuf::from(root);
    let configuration_directory = base_path.join("configurations");
    let base = configuration_directory.join("base");
    let file = configuration_directory.join(env);
    Config::builder()
        .add_source(config::File::from(base).required(true))
        .add_source(config::File::from(file).required(false))
        .add_source(config::Environment::with_prefix("APP").separator("_"))
        .build()
}
