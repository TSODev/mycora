use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    vault_path: Option<PathBuf>,
}

pub struct Config {
    pub vault_path: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let home = std::env::var("HOME").context("HOME environment variable is not set")?;
        let config_path = PathBuf::from(&home).join(".config/mycora/config.toml");

        let raw: RawConfig = if config_path.exists() {
            let text = std::fs::read_to_string(&config_path)
                .with_context(|| format!("reading {}", config_path.display()))?;
            toml::from_str(&text)
                .with_context(|| format!("parsing {}", config_path.display()))?
        } else {
            RawConfig::default()
        };

        let vault_path = raw
            .vault_path
            .unwrap_or_else(|| PathBuf::from(&home).join("mycora"));

        Ok(Self { vault_path })
    }
}
