use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

use crate::config_path;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    pub id: String,
    pub name: String,
    pub archive_source: String,
    pub enabled: bool,
    pub priority: usize,
    pub installed_files: Vec<String>,
    // Metadata (v4)
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub installed_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    // Relationships (v5)
    #[serde(default)]
    pub relationships: Vec<ModRelationEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModRelationEntry {
    pub target_mod_id: Option<String>,
    pub target_mod_name: Option<String>,
    pub relation_type: String, // "requires" | "conflicts" | "recommends" | "loads_after" | "loads_before"
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModState {
    pub mod_id: String,
    pub enabled: bool,
    pub priority: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub game_id: String,
    pub mod_states: Vec<ModState>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub game_type: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub launch_path: Option<String>,
    pub support_status: String,
    pub mods: Vec<ModEntry>,
    #[serde(default)]
    pub active_profile_id: Option<String>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub version: u32,
    pub games: Vec<GameEntry>,
}

/// Current config schema version.
/// v1 → v2: stripped mod-dir prefix from installed_files
/// v2 → v3: added profiles + active_profile_id to GameEntry
/// v3 → v4: added metadata fields to ModEntry (all Optional)
/// v4 → v5: added relationships to ModEntry (serde default = empty vec)
pub const CONFIG_VERSION: u32 = 5;

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            games: Vec::new(),
        }
    }
}

/// Migrate a v1 config: strip mod-dir prefixes.
fn migrate_v1(config: &mut AppConfig) {
    for game in &mut config.games {
        let prefix = match game.game_type.as_str() {
            "sod2" => "Content/Paks/~mods/",
            "witcher3" => "Mods/",
            _ => continue,
        };
        for m in &mut game.mods {
            for f in &mut m.installed_files {
                if let Some(stripped) = f.strip_prefix(prefix) {
                    *f = stripped.to_string();
                }
            }
        }
    }
    config.version = 2;
}

/// Migrate v2 → v3: add empty profiles vec and None active_profile_id.
fn migrate_v2(config: &mut AppConfig) {
    for game in &mut config.games {
        game.active_profile_id = None;
        game.profiles = Vec::new();
    }
    config.version = CONFIG_VERSION;
}

pub fn load() -> io::Result<AppConfig> {
    // Delegate to SQLite if DB exists, otherwise fall back to JSON migration
    crate::db::load_config()
        .or_else(|_| load_json())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

fn load_json() -> Result<AppConfig, String> {
    let path = config_path();
    if !path.exists() {
        let default = AppConfig::default();
        crate::db::save_config(&default).map_err(|e| format!("Save: {}", e))?;
        return Ok(default);
    }
    let data = fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    let mut config: AppConfig = serde_json::from_str(&data)
        .map_err(|e| format!("Config at {} is corrupt: {}", path.display(), e))?;
    if config.version < 2 { migrate_v1(&mut config); }
    if config.version < 3 { migrate_v2(&mut config); }
    if config.version < 4 { config.version = 4; }
    if config.version < 5 { config.version = CONFIG_VERSION; }
    // Save to DB
    crate::db::save_config(&config).map_err(|e| format!("DB save: {}", e))?;
    Ok(config)
}

pub fn save(config: &AppConfig) -> io::Result<()> {
    crate::db::save_config(config)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_v1_strips_mod_dir_prefixes() {
        let mut cfg: AppConfig = serde_json::from_str(r#"{
            "version": 1,
            "games": [
                {
                    "id": "g1", "type": "sod2", "name": "SoD2", "path": "/x",
                    "supportStatus": "provisional",
                    "mods": [{
                        "id": "m1", "name": "Mod", "archiveSource": "a.zip",
                        "enabled": true, "priority": 1,
                        "installedFiles": ["Content/Paks/~mods/foo.pak"]
                    }]
                },
                {
                    "id": "g2", "type": "witcher3", "name": "W3", "path": "/y",
                    "supportStatus": "verified",
                    "mods": [{
                        "id": "m2", "name": "Mod2", "archiveSource": "b.zip",
                        "enabled": true, "priority": 1,
                        "installedFiles": ["Mods/modBar/content/blob0.bundle"]
                    }]
                }
            ]
        }"#).unwrap();

        migrate_v1(&mut cfg);

        assert_eq!(cfg.version, 2); // v1→v2 migration sets version to 2
        assert_eq!(cfg.games[0].mods[0].installed_files, vec!["foo.pak"]);
        assert_eq!(cfg.games[1].mods[0].installed_files, vec!["modBar/content/blob0.bundle"]);
    }
}
