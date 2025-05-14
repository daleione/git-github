use std::env;
use std::path::Path;
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
    pub prompt: Option<String>,
}


pub fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let current_dir_config = Path::new("config.toml");
    let home = env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    let home_config = Path::new(&home).join(".config").join("git-github").join("config.toml");
    let cfg = Config::builder()
        .add_source(File::from(current_dir_config).required(false))
        .add_source(File::from(home_config).required(false))
        .build()?;
    Ok(cfg.try_deserialize()?)
}
