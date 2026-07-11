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

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            games: Vec::new(),
        }
    }
}

pub fn load() -> io::Result<AppConfig> {
    let path = config_path();
    if !path.exists() {
        let default = AppConfig::default();
        save(&default)?;
        return Ok(default);
    }
    let data = fs::read_to_string(&path)?;
    let config: AppConfig = serde_json::from_str(&data)
        .unwrap_or_default();
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
