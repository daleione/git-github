use crate::error::{Error, Result};
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
    pub model: Option<String>,
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
model = "deepseek-chat"
temperature = 0.7
prompt = ""
"#
        )?;
    }
    Ok(())
}

/// Home directory across platforms (`HOME` on Unix, `USERPROFILE` on Windows).
fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn load_config() -> Result<AppConfig> {
    // Project-local override; named specifically to avoid clashing with an
    // unrelated `config.toml` in the working directory.
    let local_config = Path::new("git-github.toml").to_path_buf();
    let home = home_dir().ok_or(Error::NoHomeDir)?;
    let home_config = home
        .join(".config")
        .join("git-github")
        .join("config.toml");

    if !local_config.exists() && !home_config.exists() {
        ensure_config_exists(&home_config)?;
    }

    let cfg = Config::builder()
        .add_source(File::from(local_config).required(false))
        .add_source(File::from(home_config).required(false))
        .build()?;

    let mut app: AppConfig = cfg.try_deserialize()?;

    // An explicit env var wins over the config file, so a key never has to be
    // written to disk (handy for CI).
    if let Some(key) = env::var("DEEPSEEK_API_KEY").ok().filter(|k| !k.is_empty()) {
        app.deepseek.api_key = key;
    }

    Ok(app)
}
