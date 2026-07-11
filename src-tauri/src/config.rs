use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

use crate::config_path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModEntry {
    pub id: String,
    pub name: String,
    pub archive_source: String,
    pub enabled: bool,
    pub priority: usize,
    pub installed_files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub version: u32,
    pub games: Vec<GameEntry>,
}

/// Current config schema version. v2 changed installed_files from
/// game-root-relative (with mod-dir prefix baked in) to mod-dir-relative.
pub const CONFIG_VERSION: u32 = 2;

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            games: Vec::new(),
        }
    }
}

/// Migrate a v1 config in place: strip the mod-dir prefix that v1 stored
/// inside each installed_files entry.
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
    config.version = CONFIG_VERSION;
}

pub fn load() -> io::Result<AppConfig> {
    let path = config_path();
    if !path.exists() {
        let default = AppConfig::default();
        save(&default)?;
        return Ok(default);
    }
    let data = fs::read_to_string(&path)?;
    // A corrupt config must surface as an error — silently resetting would
    // orphan the mod library.
    let mut config: AppConfig = serde_json::from_str(&data)
        .map_err(|e| io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Config file at {} is corrupt: {}", path.display(), e),
        ))?;
    if config.version < 2 {
        migrate_v1(&mut config);
        save(&config)?;
    }
    Ok(config)
}

pub fn save(config: &AppConfig) -> io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(config)?;
    fs::write(&path, data)?;
    Ok(())
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
                    "support_status": "provisional",
                    "mods": [{
                        "id": "m1", "name": "Mod", "archive_source": "a.zip",
                        "enabled": true, "priority": 1,
                        "installed_files": ["Content/Paks/~mods/foo.pak"]
                    }]
                },
                {
                    "id": "g2", "type": "witcher3", "name": "W3", "path": "/y",
                    "support_status": "verified",
                    "mods": [{
                        "id": "m2", "name": "Mod2", "archive_source": "b.zip",
                        "enabled": true, "priority": 1,
                        "installed_files": ["Mods/modBar/content/blob0.bundle"]
                    }]
                }
            ]
        }"#).unwrap();

        migrate_v1(&mut cfg);

        assert_eq!(cfg.version, CONFIG_VERSION);
        assert_eq!(cfg.games[0].mods[0].installed_files, vec!["foo.pak"]);
        assert_eq!(cfg.games[1].mods[0].installed_files, vec!["modBar/content/blob0.bundle"]);
    }
}
