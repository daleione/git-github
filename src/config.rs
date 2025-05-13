use serde::Deserialize;
use config::{Config, File};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub deepseek: DeepSeekConfig,
}

#[derive(Debug, Deserialize)]
pub struct DeepSeekConfig {
    pub api_key: String,
    pub temperature: Option<f32>,
}

pub fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let cfg = Config::builder()
        .add_source(File::with_name("config"))
        .build()?;
    Ok(cfg.try_deserialize()?)
}
