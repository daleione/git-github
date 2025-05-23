use config::{Config, File};
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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

fn ensure_config_exists(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(path)?;
        writeln!(
            file,
            r#"[deepseek]
api_key = ""
temperature = 0.7
prompt = """"
"#
        )?;
    }
    Ok(())
}

pub fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let current_dir_config = Path::new("config.toml").to_path_buf();
    let home = env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    let home_config = PathBuf::from(&home)
        .join(".config")
        .join("git-github")
        .join("config.toml");

    if !current_dir_config.exists() && !home_config.exists() {
        ensure_config_exists(&home_config)?;
    }

    let cfg = Config::builder()
        .add_source(File::from(current_dir_config).required(false))
        .add_source(File::from(home_config).required(false))
        .build()?;

    Ok(cfg.try_deserialize()?)
}
