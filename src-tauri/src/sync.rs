use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "unknown".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncManifest {
    pub schema_version: u32,
    pub machine_id: String,
    pub last_synced: String,
    pub games: Vec<SyncGame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncGame {
    pub game_type: String,
    pub machine_path_hint: String,
    pub active_profile_id: Option<String>,
    pub profiles: Vec<SyncProfile>,
    pub mods: Vec<SyncMod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProfile {
    pub name: String,
    pub mod_states: Vec<SyncModState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncModState {
    pub mod_name: String,
    pub enabled: bool,
    pub priority: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncMod {
    pub name: String,
    pub version: Option<String>,
    pub enabled: bool,
    pub priority: usize,
    pub source_url: Option<String>,
    pub category: Option<String>,
}

/// Export current config as a machine-independent sync manifest.
pub fn export_manifest(cfg: &AppConfig) -> SyncManifest {
    let now = crate::format_ts(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    );

    SyncManifest {
        schema_version: 1,
        machine_id: hostname(),
        last_synced: now,
        games: cfg.games.iter().map(|game| SyncGame {
            game_type: game.game_type.clone(),
            machine_path_hint: game.path.clone(),
            active_profile_id: game.active_profile_id.clone(),
            profiles: game.profiles.iter().map(|p| SyncProfile {
                name: p.name.clone(),
                mod_states: p.mod_states.iter().map(|ms| {
                    let mod_name = game.mods.iter()
                        .find(|m| m.id == ms.mod_id)
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| "unknown".to_string());
                    SyncModState { mod_name, enabled: ms.enabled, priority: ms.priority }
                }).collect(),
            }).collect(),
            mods: game.mods.iter().map(|m| SyncMod {
                name: m.name.clone(),
                version: m.version.clone(),
                enabled: m.enabled,
                priority: m.priority,
                source_url: m.source_url.clone(),
                category: m.category.clone(),
            }).collect(),
        }).collect(),
    }
}

/// Apply a sync manifest: update mod enabled states and priorities to match.
/// Only affects mods that exist on both sides (matched by name).
pub fn import_manifest(cfg: &mut AppConfig, manifest: &SyncManifest) -> (usize, usize) {
    let mut applied = 0usize;
    let mut skipped = 0usize;

    for sync_game in &manifest.games {
        if let Some(game) = cfg.games.iter_mut().find(|g| g.game_type == sync_game.game_type) {
            for sync_mod in &sync_game.mods {
                if let Some(mod_entry) = game.mods.iter_mut().find(|m| m.name == sync_mod.name) {
                    mod_entry.enabled = sync_mod.enabled;
                    mod_entry.priority = sync_mod.priority;
                    applied += 1;
                } else {
                    skipped += 1;
                }
            }
            // Apply profiles
            for sync_profile in &sync_game.profiles {
                if let Some(profile) = game.profiles.iter_mut().find(|p| p.name == sync_profile.name) {
                    for ms in &sync_profile.mod_states {
                        if let Some(mod_entry) = game.mods.iter().find(|m| m.name == ms.mod_name) {
                            if let Some(pms) = profile.mod_states.iter_mut().find(|pms| pms.mod_id == mod_entry.id) {
                                pms.enabled = ms.enabled;
                                pms.priority = ms.priority;
                            }
                        }
                    }
                }
            }
        }
    }

    (applied, skipped)
}
