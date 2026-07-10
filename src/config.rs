use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// A vault known to Mycora: a name (unique within the registry, used to
/// address the vault before multi-vault mounting has UI of its own) and the
/// on-disk directory `Vault::open` should point at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultEntry {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    /// Pre-registry single-vault key. Still honored when `vaults` is empty,
    /// so an existing `config.toml` from before the registry keeps working
    /// unchanged rather than silently reverting to `~/mycora`.
    vault_path: Option<PathBuf>,
    #[serde(default)]
    vaults: Vec<RawVaultEntry>,
}

#[derive(Debug, Deserialize)]
struct RawVaultEntry {
    name: String,
    path: PathBuf,
}

pub struct Config {
    /// Every vault the registry knows about. Always non-empty: falls back to
    /// a single `"default"` entry at `~/mycora` (or `vault_path`, see above)
    /// when the config declares none.
    pub vaults: Vec<VaultEntry>,
    /// The resolved `$HOME`, kept around so callers that need an XDG-style
    /// path (e.g. `Index::default_path`) don't each re-read the env var.
    pub home: String,
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

        Self::from_raw(raw, &home)
    }

    fn from_raw(raw: RawConfig, home: &str) -> Result<Self> {
        let vaults = if raw.vaults.is_empty() {
            let path = raw
                .vault_path
                .unwrap_or_else(|| PathBuf::from(home).join("mycora"));
            vec![VaultEntry {
                name: "default".to_string(),
                path,
            }]
        } else {
            raw.vaults
                .into_iter()
                .map(|v| VaultEntry {
                    name: v.name,
                    path: v.path,
                })
                .collect()
        };

        let mut seen = HashSet::new();
        for entry in &vaults {
            if !seen.insert(entry.name.as_str()) {
                bail!("duplicate vault name in config: \"{}\"", entry.name);
            }
        }

        Ok(Self {
            vaults,
            home: home.to_string(),
        })
    }

    /// The vault to open on startup. Until App-level mounting exists (only
    /// the registry is implemented so far — see ROADMAP.md's "Multiple
    /// vaults" entry), Mycora always runs against exactly one: the entry
    /// named `"default"` if the registry has one, else the first entry.
    pub fn active_vault(&self) -> &VaultEntry {
        self.vaults
            .iter()
            .find(|v| v.name == "default")
            .unwrap_or(&self.vaults[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_config_defaults_to_a_single_default_vault_under_home() {
        let config = Config::from_raw(RawConfig::default(), "/home/alice").unwrap();
        assert_eq!(
            config.vaults,
            vec![VaultEntry {
                name: "default".to_string(),
                path: PathBuf::from("/home/alice/mycora"),
            }]
        );
    }

    #[test]
    fn legacy_vault_path_still_works_when_no_registry_is_present() {
        let raw = RawConfig {
            vault_path: Some(PathBuf::from("/custom/vault")),
            vaults: Vec::new(),
        };
        let config = Config::from_raw(raw, "/home/alice").unwrap();
        assert_eq!(config.vaults.len(), 1);
        assert_eq!(config.vaults[0].name, "default");
        assert_eq!(config.vaults[0].path, PathBuf::from("/custom/vault"));
    }

    #[test]
    fn registry_entries_take_priority_over_legacy_vault_path() {
        let raw = RawConfig {
            vault_path: Some(PathBuf::from("/ignored")),
            vaults: vec![RawVaultEntry {
                name: "work".to_string(),
                path: PathBuf::from("/vaults/work"),
            }],
        };
        let config = Config::from_raw(raw, "/home/alice").unwrap();
        assert_eq!(config.vaults.len(), 1);
        assert_eq!(config.vaults[0].name, "work");
    }

    #[test]
    fn duplicate_vault_names_are_rejected() {
        let raw = RawConfig {
            vault_path: None,
            vaults: vec![
                RawVaultEntry {
                    name: "work".to_string(),
                    path: PathBuf::from("/vaults/work"),
                },
                RawVaultEntry {
                    name: "work".to_string(),
                    path: PathBuf::from("/vaults/other"),
                },
            ],
        };
        assert!(Config::from_raw(raw, "/home/alice").is_err());
    }

    #[test]
    fn active_vault_prefers_the_entry_named_default() {
        let raw = RawConfig {
            vault_path: None,
            vaults: vec![
                RawVaultEntry {
                    name: "work".to_string(),
                    path: PathBuf::from("/vaults/work"),
                },
                RawVaultEntry {
                    name: "default".to_string(),
                    path: PathBuf::from("/vaults/default"),
                },
            ],
        };
        let config = Config::from_raw(raw, "/home/alice").unwrap();
        assert_eq!(config.active_vault().name, "default");
    }

    #[test]
    fn active_vault_falls_back_to_the_first_entry_when_none_is_named_default() {
        let raw = RawConfig {
            vault_path: None,
            vaults: vec![
                RawVaultEntry {
                    name: "work".to_string(),
                    path: PathBuf::from("/vaults/work"),
                },
                RawVaultEntry {
                    name: "personal".to_string(),
                    path: PathBuf::from("/vaults/personal"),
                },
            ],
        };
        let config = Config::from_raw(raw, "/home/alice").unwrap();
        assert_eq!(config.active_vault().name, "work");
    }
}
