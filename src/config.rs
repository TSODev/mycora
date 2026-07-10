use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// A vault known to Mycora: a name (unique within the registry, doubling as
/// the index's `vault_id`) and the on-disk directory `Vault::open` should
/// point at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultEntry {
    pub name: String,
    pub path: PathBuf,
    /// Whether `App`/the CLI reindex commands should load this vault at
    /// startup. `true` by default — a registry entry only stays *known but
    /// inactive* if the user explicitly opts it out with `mounted = false`.
    pub mounted: bool,
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
    #[serde(default = "default_mounted")]
    mounted: bool,
}

fn default_mounted() -> bool {
    true
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
                mounted: true,
            }]
        } else {
            raw.vaults
                .into_iter()
                .map(|v| VaultEntry {
                    name: v.name,
                    path: v.path,
                    mounted: v.mounted,
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

    /// Every vault flagged `mounted` in the registry — what `App` and the
    /// CLI reindex commands actually load. A registry can hold vaults that
    /// aren't currently mounted (`mounted = false`); see ROADMAP.md's
    /// "Multiple vaults" entry.
    pub fn mounted_vaults(&self) -> impl Iterator<Item = &VaultEntry> {
        self.vaults.iter().filter(|v| v.mounted)
    }

    /// The single *editable* vault: the entry named `"default"` among the
    /// mounted ones if there is one, else the first mounted entry. Every
    /// other mounted vault is read-only in the TUI for now — full
    /// multi-vault editing needs every mutating `App` method to resolve
    /// which vault a note belongs to first, deferred to a later pass (see
    /// ROADMAP.md's "Multiple vaults" entry).
    pub fn active_vault(&self) -> &VaultEntry {
        let mounted: Vec<&VaultEntry> = self.mounted_vaults().collect();
        // Self-heal rather than fail to start if every entry opted out of
        // mounting: the app always needs at least one editable vault.
        let candidates: Vec<&VaultEntry> = if mounted.is_empty() {
            self.vaults.iter().collect()
        } else {
            mounted
        };
        candidates
            .iter()
            .find(|v| v.name == "default")
            .copied()
            .unwrap_or(candidates[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_vault(name: &str, path: &str) -> RawVaultEntry {
        RawVaultEntry {
            name: name.to_string(),
            path: PathBuf::from(path),
            mounted: true,
        }
    }

    #[test]
    fn no_config_defaults_to_a_single_default_vault_under_home() {
        let config = Config::from_raw(RawConfig::default(), "/home/alice").unwrap();
        assert_eq!(
            config.vaults,
            vec![VaultEntry {
                name: "default".to_string(),
                path: PathBuf::from("/home/alice/mycora"),
                mounted: true,
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
            vaults: vec![raw_vault("work", "/vaults/work")],
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
                raw_vault("work", "/vaults/work"),
                raw_vault("work", "/vaults/other"),
            ],
        };
        assert!(Config::from_raw(raw, "/home/alice").is_err());
    }

    #[test]
    fn active_vault_prefers_the_entry_named_default() {
        let raw = RawConfig {
            vault_path: None,
            vaults: vec![
                raw_vault("work", "/vaults/work"),
                raw_vault("default", "/vaults/default"),
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
                raw_vault("work", "/vaults/work"),
                raw_vault("personal", "/vaults/personal"),
            ],
        };
        let config = Config::from_raw(raw, "/home/alice").unwrap();
        assert_eq!(config.active_vault().name, "work");
    }

    #[test]
    fn mounted_defaults_to_true_when_omitted() {
        let raw = RawConfig {
            vault_path: None,
            vaults: vec![raw_vault("work", "/vaults/work")],
        };
        let config = Config::from_raw(raw, "/home/alice").unwrap();
        assert!(config.vaults[0].mounted);
    }

    #[test]
    fn mounted_vaults_excludes_entries_with_mounted_false() {
        let mut archive = raw_vault("archive", "/vaults/archive");
        archive.mounted = false;
        let raw = RawConfig {
            vault_path: None,
            vaults: vec![raw_vault("default", "/vaults/default"), archive],
        };
        let config = Config::from_raw(raw, "/home/alice").unwrap();
        let mounted: Vec<&str> = config.mounted_vaults().map(|v| v.name.as_str()).collect();
        assert_eq!(mounted, vec!["default"]);
    }

    #[test]
    fn active_vault_self_heals_when_nothing_is_mounted() {
        let mut work = raw_vault("work", "/vaults/work");
        work.mounted = false;
        let raw = RawConfig {
            vault_path: None,
            vaults: vec![work],
        };
        let config = Config::from_raw(raw, "/home/alice").unwrap();
        // Every entry opted out of mounting, but active_vault() still needs
        // to return *something* rather than panicking.
        assert_eq!(config.active_vault().name, "work");
    }
}
