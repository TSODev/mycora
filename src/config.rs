use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Default, Deserialize, Serialize)]
struct RawConfig {
    /// Pre-registry single-vault key. Still honored when `vaults` is empty,
    /// so an existing `config.toml` from before the registry keeps working
    /// unchanged rather than silently reverting to `~/mycora`.
    #[serde(skip_serializing_if = "Option::is_none")]
    vault_path: Option<PathBuf>,
    #[serde(default)]
    vaults: Vec<RawVaultEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
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
    /// `~/.config/mycora/config.toml` — kept as its own method (mirroring
    /// `Session::default_path`/`Index::default_path`) so callers that need
    /// the path without loading a full `Config` (e.g. `add_vault`, and its
    /// own tests) don't have to duplicate the join.
    pub fn default_path(home: &str) -> PathBuf {
        PathBuf::from(home).join(".config/mycora/config.toml")
    }

    pub fn load() -> Result<Self> {
        let home = std::env::var("HOME").context("HOME environment variable is not set")?;
        let config_path = Self::default_path(&home);

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

    /// Registers a new vault at `config_path` (`Self::default_path`'s
    /// result, for the real `mycora vault add` CLI command — kept as an
    /// explicit parameter rather than resolving `HOME` internally so this
    /// stays testable against a scratch path, matching every other
    /// path-taking method in the crate). Creates the file (and its parent
    /// directory) if neither exists yet. Errors rather than silently
    /// overwriting if `name` is already registered; remove the old entry
    /// first (by hand, or `vault_rename` it out of the way) if replacing
    /// it is what's wanted.
    ///
    /// Rewrites the whole file from a fresh parse — like `cargo add`
    /// rewriting `Cargo.toml` — rather than a surgical text insertion.
    /// Simpler, but loses hand-added comments/formatting in the file;
    /// config.toml is edited rarely enough that this is an acceptable
    /// tradeoff for now (shared by every `*_vault` method below, via
    /// `read_raw`/`write_raw`). If the file only had the legacy
    /// `vault_path` key (no `vaults` registry yet), that implicit
    /// `"default"` vault is migrated into an explicit registry entry
    /// first, so adding a second vault doesn't silently drop the first
    /// one.
    pub fn add_vault(config_path: &Path, name: &str, path: PathBuf, mounted: bool) -> Result<()> {
        let mut raw = read_raw(config_path)?;

        if raw.vaults.iter().any(|v| v.name == name) {
            bail!(
                "a vault named \"{name}\" is already registered in {}",
                config_path.display()
            );
        }

        migrate_legacy_vault_path(&mut raw);

        raw.vaults.push(RawVaultEntry {
            name: name.to_string(),
            path,
            mounted,
        });

        write_raw(config_path, &raw)
    }

    /// Renames a registered vault from `old_name` to `new_name`. A no-op
    /// if the two are equal (returns `Ok` without touching the file).
    /// Errors if `old_name` isn't registered, or if `new_name` is already
    /// taken by a *different* entry. Path and `mounted` are untouched —
    /// only the name changes, so this is also how you free up `"default"`
    /// for `promote_vault` to reassign to another vault.
    pub fn rename_vault(config_path: &Path, old_name: &str, new_name: &str) -> Result<()> {
        if old_name == new_name {
            return Ok(());
        }

        let mut raw = read_raw(config_path)?;
        migrate_legacy_vault_path(&mut raw);

        if !raw.vaults.iter().any(|v| v.name == old_name) {
            bail!("no vault named \"{old_name}\" in {}", config_path.display());
        }
        if raw.vaults.iter().any(|v| v.name == new_name) {
            bail!(
                "a vault named \"{new_name}\" is already registered in {}",
                config_path.display()
            );
        }

        for entry in &mut raw.vaults {
            if entry.name == old_name {
                entry.name = new_name.to_string();
            }
        }

        write_raw(config_path, &raw)
    }

    /// Makes `name` the active/editable vault (`Config::active_vault`) by
    /// renaming it to `"default"` — the name that method looks for. A
    /// no-op if it's already named `"default"`. Errors if `name` isn't
    /// registered, or if a *different* vault already holds the
    /// `"default"` name — deliberately doesn't reassign that one itself
    /// (confirmed with the user before implementing, same question
    /// `vault_init` raised): rename it out of the way first with
    /// `rename_vault(config_path, "default", "something-else")`, then
    /// retry. Keeps this operation narrow and composed from
    /// `rename_vault` rather than silently touching an entry the caller
    /// didn't name.
    pub fn promote_vault(config_path: &Path, name: &str) -> Result<()> {
        let mut raw = read_raw(config_path)?;
        migrate_legacy_vault_path(&mut raw);

        if !raw.vaults.iter().any(|v| v.name == name) {
            bail!("no vault named \"{name}\" in {}", config_path.display());
        }
        if name == "default" {
            return Ok(());
        }
        if raw.vaults.iter().any(|v| v.name == "default") {
            bail!(
                "a vault named \"default\" is already registered in {} — rename it first with \
                 `mycora vault rename default <new-name>`, then retry `mycora vault promote {name}`",
                config_path.display()
            );
        }

        for entry in &mut raw.vaults {
            if entry.name == name {
                entry.name = "default".to_string();
            }
        }

        write_raw(config_path, &raw)
    }
}

/// Reads and parses `config_path`, or an empty `RawConfig` if it doesn't
/// exist yet — shared by every `Config::*_vault` writer method.
fn read_raw(config_path: &Path) -> Result<RawConfig> {
    if config_path.exists() {
        let text = std::fs::read_to_string(config_path)
            .with_context(|| format!("reading {}", config_path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", config_path.display()))
    } else {
        Ok(RawConfig::default())
    }
}

/// Serializes and writes `raw` to `config_path`, creating its parent
/// directory first if needed — shared by every `Config::*_vault` writer
/// method.
fn write_raw(config_path: &Path, raw: &RawConfig) -> Result<()> {
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let text = toml::to_string_pretty(raw).context("serializing config.toml")?;
    std::fs::write(config_path, text)
        .with_context(|| format!("writing {}", config_path.display()))
}

/// If the registry is empty but a legacy `vault_path` key is set, turns
/// that implicit vault into an explicit `"default"` entry — called before
/// every registry write so an old single-vault config isn't silently
/// dropped the first time a `vault add`/`rename`/`promote` command
/// touches the file.
fn migrate_legacy_vault_path(raw: &mut RawConfig) {
    if raw.vaults.is_empty()
        && let Some(legacy_path) = raw.vault_path.take()
    {
        raw.vaults.push(RawVaultEntry {
            name: "default".to_string(),
            path: legacy_path,
            mounted: true,
        });
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

    fn scratch_config_path() -> PathBuf {
        std::env::temp_dir().join(format!("mycora-config-test-{}.toml", uuid::Uuid::new_v4()))
    }

    #[test]
    fn add_vault_creates_the_file_and_parent_dir_when_neither_exists() {
        let dir = std::env::temp_dir().join(format!("mycora-config-test-{}", uuid::Uuid::new_v4()));
        let config_path = dir.join("config.toml");
        assert!(!dir.exists());

        Config::add_vault(&config_path, "work", PathBuf::from("/vaults/work"), true).unwrap();

        let config = Config::from_raw(
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap(),
            "/home/alice",
        )
        .unwrap();
        assert_eq!(config.vaults.len(), 1);
        assert_eq!(config.vaults[0].name, "work");
        assert_eq!(config.vaults[0].path, PathBuf::from("/vaults/work"));
        assert!(config.vaults[0].mounted);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn add_vault_preserves_existing_entries() {
        let path = scratch_config_path();
        Config::add_vault(&path, "work", PathBuf::from("/vaults/work"), true).unwrap();
        Config::add_vault(&path, "archive", PathBuf::from("/vaults/archive"), false).unwrap();

        let raw: RawConfig = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let config = Config::from_raw(raw, "/home/alice").unwrap();
        assert_eq!(config.vaults.len(), 2);
        assert_eq!(config.vaults[0].name, "work");
        assert!(config.vaults[0].mounted);
        assert_eq!(config.vaults[1].name, "archive");
        assert_eq!(config.vaults[1].path, PathBuf::from("/vaults/archive"));
        assert!(!config.vaults[1].mounted);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn add_vault_rejects_a_duplicate_name() {
        let path = scratch_config_path();
        Config::add_vault(&path, "work", PathBuf::from("/vaults/work"), true).unwrap();

        let err = Config::add_vault(&path, "work", PathBuf::from("/vaults/other"), true)
            .unwrap_err();
        assert!(err.to_string().contains("already registered"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn add_vault_migrates_a_legacy_vault_path_into_the_registry() {
        let path = scratch_config_path();
        std::fs::write(&path, "vault_path = \"/legacy/vault\"\n").unwrap();

        Config::add_vault(&path, "second", PathBuf::from("/vaults/second"), true).unwrap();

        let raw: RawConfig = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let config = Config::from_raw(raw, "/home/alice").unwrap();
        assert_eq!(config.vaults.len(), 2);
        assert_eq!(config.vaults[0].name, "default");
        assert_eq!(config.vaults[0].path, PathBuf::from("/legacy/vault"));
        assert_eq!(config.vaults[1].name, "second");

        std::fs::remove_file(&path).ok();
    }

    fn config_at(path: &std::path::Path) -> Config {
        let raw: RawConfig = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        Config::from_raw(raw, "/home/alice").unwrap()
    }

    #[test]
    fn rename_vault_updates_the_name_in_place() {
        let path = scratch_config_path();
        Config::add_vault(&path, "work", PathBuf::from("/vaults/work"), true).unwrap();

        Config::rename_vault(&path, "work", "personal").unwrap();

        let config = config_at(&path);
        assert_eq!(config.vaults.len(), 1);
        assert_eq!(config.vaults[0].name, "personal");
        assert_eq!(config.vaults[0].path, PathBuf::from("/vaults/work"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn rename_vault_same_name_is_a_noop() {
        let path = scratch_config_path();
        Config::add_vault(&path, "work", PathBuf::from("/vaults/work"), true).unwrap();

        Config::rename_vault(&path, "work", "work").unwrap();

        assert_eq!(config_at(&path).vaults[0].name, "work");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn rename_vault_errors_if_old_name_not_found() {
        let path = scratch_config_path();
        Config::add_vault(&path, "work", PathBuf::from("/vaults/work"), true).unwrap();

        let err = Config::rename_vault(&path, "nope", "personal").unwrap_err();
        assert!(err.to_string().contains("no vault named"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn rename_vault_errors_if_new_name_already_taken() {
        let path = scratch_config_path();
        Config::add_vault(&path, "work", PathBuf::from("/vaults/work"), true).unwrap();
        Config::add_vault(&path, "personal", PathBuf::from("/vaults/personal"), true).unwrap();

        let err = Config::rename_vault(&path, "work", "personal").unwrap_err();
        assert!(err.to_string().contains("already registered"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn promote_vault_renames_the_target_to_default() {
        let path = scratch_config_path();
        Config::add_vault(&path, "work", PathBuf::from("/vaults/work"), true).unwrap();

        Config::promote_vault(&path, "work").unwrap();

        let config = config_at(&path);
        assert_eq!(config.active_vault().name, "default");
        assert_eq!(config.active_vault().path, PathBuf::from("/vaults/work"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn promote_vault_already_default_is_a_noop() {
        let path = scratch_config_path();
        Config::add_vault(&path, "default", PathBuf::from("/vaults/default"), true).unwrap();

        Config::promote_vault(&path, "default").unwrap();

        assert_eq!(config_at(&path).vaults.len(), 1);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn promote_vault_errors_if_name_not_found() {
        let path = scratch_config_path();
        Config::add_vault(&path, "work", PathBuf::from("/vaults/work"), true).unwrap();

        let err = Config::promote_vault(&path, "nope").unwrap_err();
        assert!(err.to_string().contains("no vault named"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn promote_vault_errors_if_a_different_default_already_exists() {
        let path = scratch_config_path();
        Config::add_vault(&path, "default", PathBuf::from("/vaults/default"), true).unwrap();
        Config::add_vault(&path, "work", PathBuf::from("/vaults/work"), true).unwrap();

        let err = Config::promote_vault(&path, "work").unwrap_err();
        assert!(err.to_string().contains("rename it first"));

        // Nothing was renamed on failure.
        let config = config_at(&path);
        assert_eq!(config.active_vault().name, "default");
        assert_eq!(config.active_vault().path, PathBuf::from("/vaults/default"));

        std::fs::remove_file(&path).ok();
    }
}
