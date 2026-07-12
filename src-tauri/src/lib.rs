use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

mod archive;
mod config;
mod conflicts;
mod db;
mod dependencies;
mod deploy;
mod detection;
mod downloader;
mod games;
mod plugins;
mod sync;
mod update_check;

pub use config::AppConfig;
pub use games::{Game, GameRegistry, GameType};

/// Returns the config directory: ~/.config/linux-mod-manager/
fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("linux-mod-manager")
}

/// Returns the library directory: ~/.config/linux-mod-manager/library/
fn library_dir() -> PathBuf {
    config_dir().join("library")
}

/// Returns the config file path: ~/.config/linux-mod-manager/config.json
fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn ensure_dirs() -> std::io::Result<()> {
    fs::create_dir_all(config_dir())?;
    fs::create_dir_all(library_dir())?;
    Ok(())
}

// ═══════════════════════════════════════════════════════
// Tauri IPC Commands
// ═══════════════════════════════════════════════════════

#[tauri::command]
fn get_game_definitions() -> Result<Vec<games::GameDefinition>, String> {
    Ok(games::all_definitions().into_iter().cloned().collect())
}

#[tauri::command]
fn scan_for_games() -> Result<Vec<detection::DetectedGame>, String> {
    Ok(detection::scan_steam())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptExtenderStatus {
    pub game_type: String,
    pub extender_name: String,
    pub short_name: String,
    pub is_installed: bool,
    pub website: String,
    pub is_launcher: bool,
}

#[tauri::command]
fn scan_plugins(game_id: String) -> Result<Vec<plugins::PluginInfo>, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg.games.iter().find(|g| g.id == game_id).ok_or("Game not found")?;
    let data_dir = std::path::PathBuf::from(&game.path).join("Data");
    plugins::scan_plugins(&data_dir)
}

#[tauri::command]
fn validate_plugins(plugins: Vec<plugins::PluginInfo>) -> Result<plugins::PluginValidation, String> {
    Ok(plugins::validate_plugins(&plugins))
}

#[tauri::command]
fn check_script_extenders(game_id: String) -> Result<Option<ScriptExtenderStatus>, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg.games.iter().find(|g| g.id == game_id).ok_or("Game not found")?;
    let install_path = PathBuf::from(&game.path);

    for (gt, spec) in games::get_script_extenders() {
        if gt == game.game_type {
            let installed = spec.required_files.iter().any(|f| install_path.join(f).exists());
            return Ok(Some(ScriptExtenderStatus {
                game_type: game.game_type.clone(),
                extender_name: spec.name.clone(),
                short_name: spec.short_name.clone(),
                is_installed: installed,
                website: spec.website.clone(),
                is_launcher: spec.is_launcher,
            }));
        }
    }
    Ok(None) // No extender defined for this game
}

#[tauri::command]
fn start_download(
    game_id: String,
    game_type: String,
    mod_name: String,
    url: String,
    filename: String,
) -> Result<downloader::DownloadJob, String> {
    downloader::start_download(game_id, game_type, mod_name, url, filename)
}

#[tauri::command]
fn cancel_download(job_id: String) -> Result<(), String> {
    downloader::cancel_download(job_id)
}

#[tauri::command]
fn get_download_status() -> Result<Vec<downloader::DownloadJob>, String> {
    downloader::get_download_status()
}

#[tauri::command]
fn clear_completed_downloads() -> Result<(), String> {
    downloader::clear_completed_downloads()
}

#[tauri::command]
fn export_sync_manifest() -> Result<sync::SyncManifest, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    Ok(sync::export_manifest(&cfg))
}

#[tauri::command]
fn import_sync_manifest(manifest: sync::SyncManifest) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let (applied, skipped) = sync::import_manifest(&mut cfg, &manifest);
    config::save(&cfg).map_err(|e| e.to_string())?;
    // Redeploy all games
    for game in &cfg.games {
        let _ = deploy::deploy_mods(game);
    }
    Ok(cfg)
}

#[tauri::command]
async fn check_updates(game_id: String) -> Result<Vec<update_check::UpdateResult>, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    update_check::check_updates(&game_id, &cfg).await
}

#[tauri::command]
fn resolve_dependencies(
    game_id: String,
    mod_id: String,
) -> Result<dependencies::DependencyResult, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg.games.iter().find(|g| g.id == game_id).ok_or("Game not found")?;
    let mod_entry = game.mods.iter().find(|m| m.id == mod_id).ok_or("Mod not found")?;
    Ok(dependencies::resolve_enable(mod_entry, game))
}

#[tauri::command]
fn get_config() -> Result<AppConfig, String> {
    config::load().map_err(|e| e.to_string())
}

#[tauri::command]
fn add_game(name: String, path: String, game_type: String) -> Result<AppConfig, String> {
    let gt = GameType::from_str(&game_type);

    // Verify the path before saving
    deploy::verify_game_path(&gt, &PathBuf::from(&path))?;

    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let support_status = GameRegistry::get(&gt).support_status();
    let game = config::GameEntry {
        id: uuid::Uuid::new_v4().to_string(),
        game_type: game_type.clone(),
        name,
        path,
        launch_path: None,
        support_status: support_status.to_string(),
        mods: vec![],
        active_profile_id: None,
        profiles: vec![],
    };
    cfg.games.push(game);
    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
fn remove_game(game_id: String) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;

    // Remove symlinks before removing game
    if let Some(game) = cfg.games.iter().find(|g| g.id == game_id) {
        let _ = deploy::remove_all_symlinks(game);
    }

    cfg.games.retain(|g| g.id != game_id);
    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub mod_id: String,
    pub mod_name: String,
    pub installed_files: Vec<String>,
    pub warning: Option<String>,
}

#[tauri::command]
fn import_mod(
    game_id: String,
    archive_path: String,
    mod_name: String,
) -> Result<ImportResult, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;

    // Check for duplicates
    if let Some(existing) = game.mods.iter().find(|m| m.name == mod_name) {
        return Err(format!(
            "A mod named '{}' already exists for this game. Use Update Mod to replace it, or choose a different name.",
            existing.name
        ));
    }

    // Extract the archive
    let archive_path = PathBuf::from(&archive_path);
    let (extract_dir, _extracted_files) =
        archive::extract_archive(&archive_path, &game.game_type, &mod_name)?;

    // Validate format based on game type
    let installed_files = match game.game_type.as_str() {
        "sod2" => {
            let _pak_files = archive::validate_sod2(&extract_dir)?;
            archive::derive_installed_files(&extract_dir, "sod2", &mod_name)?
        }
        "witcher3" => {
            let mod_root = archive::validate_witcher3(&extract_dir)?;
            archive::derive_installed_files(&mod_root, "witcher3", &mod_name)?
        }
        // Generic games: use definition-based validation
        other => {
            let def = games::get_definition(other)
                .ok_or_else(|| format!("Unknown game type: {}", other))?;
            match def.validation.mode.as_str() {
                "containsFileType" => {
                    let ext = def.validation.value.as_str();
                    let found = archive::validate_by_extension(&extract_dir, ext)?;
                    if found.is_empty() {
                        return Err(format!("No {} files found in archive", ext));
                    }
                }
                "containsSubdirectory" => {
                    let subdir = def.validation.value.as_str();
                    archive::validate_subdirectory(&extract_dir, subdir)?;
                }
                _ => {
                    // "anyFiles" — accept anything, derive all files
                }
            }
            archive::derive_installed_files(&extract_dir, other, &mod_name)?
        }
    };

    if installed_files.is_empty() {
        return Err("No files found to install. The archive may be empty or malformed.".to_string());
    }

    // Add mod to config (disabled by default)
    let priority = game.mods.len() + 1;
    let mod_id = uuid::Uuid::new_v4().to_string();

    // Extract version from archive filename
    let archive_name = archive_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let extracted_version = archive::extract_version_from_filename(&archive_name);

    // Timestamp
    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mod_entry = config::ModEntry {
        id: mod_id.clone(),
        name: mod_name.clone(),
        archive_source: archive_name,
        enabled: false,
        priority,
        installed_files: installed_files.clone(),
        version: extracted_version,
        author: None,
        description: None,
        source_url: None,
        category: None,
        tags: vec![],
        installed_at: Some(format_ts(now_ts)),
        updated_at: None,
        relationships: vec![],
    };

    let game = cfg.games.iter_mut().find(|g| g.id == game_id).unwrap();
    game.mods.push(mod_entry);
    config::save(&cfg).map_err(|e| e.to_string())?;

    Ok(ImportResult {
        mod_id,
        mod_name,
        installed_files,
        warning: None,
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleResult {
    pub success: bool,
    pub conflict: Option<ConflictInfo>,
    pub deploy_results: Option<Vec<(String, bool, String)>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictInfo {
    pub level: String, // "warn", "block", or "none"
    pub conflicts_with: Vec<String>,
    pub overlapping_files: Vec<String>,
}

#[tauri::command]
fn toggle_mod(game_id: String, mod_id: String) -> Result<ToggleResult, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;

    let game_idx = cfg
        .games
        .iter()
        .position(|g| g.id == game_id)
        .ok_or("Game not found")?;

    let has_load_order = {
        let gt: GameType = serde_json::from_str(&format!("\"{}\"", cfg.games[game_idx].game_type))
            .map_err(|e| format!("Unknown game type: {}", e))?;
        GameRegistry::get(&gt).has_load_order()
    };
    let game_name = cfg.games[game_idx].name.clone();
    let game_type = cfg.games[game_idx].game_type.clone();
    let game_path = cfg.games[game_idx].path.clone();

    let new_enabled = {
        let mod_entry = cfg.games[game_idx]
            .mods
            .iter()
            .find(|m| m.id == mod_id)
            .ok_or("Mod not found")?;
        !mod_entry.enabled
    };

    // If enabling, run conflict check first
    if new_enabled {
        let mod_entry = cfg.games[game_idx].mods.iter().find(|m| m.id == mod_id).unwrap();
        let mut check_entry = mod_entry.clone();
        check_entry.enabled = true;

        let enabled_mods: Vec<config::ModEntry> = cfg.games[game_idx]
            .mods
            .iter()
            .filter(|m| m.id != mod_id && m.enabled)
            .cloned()
            .collect();

        let conflict = conflicts::check_enable_conflict(
            &check_entry,
            &enabled_mods,
            has_load_order,
        );

        if conflict.level == "block" {
            return Ok(ToggleResult {
                success: false,
                conflict: Some(ConflictInfo {
                    level: conflict.level,
                    conflicts_with: conflict.conflicts_with,
                    overlapping_files: conflict.overlapping_files,
                }),
                deploy_results: None,
            });
        }

        // Apply toggle
        cfg.games[game_idx].mods.iter_mut().find(|m| m.id == mod_id).unwrap().enabled = true;
        config::save(&cfg).map_err(|e| e.to_string())?;

        // Build a temporary game entry for deploy
        let game = config::GameEntry {
            id: game_id.clone(),
            game_type: game_type.clone(),
            name: game_name,
            path: game_path,
            launch_path: None,
            support_status: cfg.games[game_idx].support_status.clone(),
            mods: cfg.games[game_idx].mods.clone(),
            active_profile_id: None,
            profiles: vec![],
        };
        let deploy_results = deploy::deploy_mods(&game).ok();

        return Ok(ToggleResult {
            success: true,
            conflict: if conflict.level == "warn" {
                Some(ConflictInfo {
                    level: conflict.level,
                    conflicts_with: conflict.conflicts_with,
                    overlapping_files: conflict.overlapping_files,
                })
            } else {
                None
            },
            deploy_results,
        });
    }

    // Disabling — apply toggle
    cfg.games[game_idx].mods.iter_mut().find(|m| m.id == mod_id).unwrap().enabled = false;

    // Renumber priorities for Witcher 3 (keep contiguous, no gaps)
    if has_load_order {
        let mut enabled: Vec<&mut config::ModEntry> = cfg.games[game_idx]
            .mods.iter_mut()
            .filter(|m| m.enabled)
            .collect();
        enabled.sort_by_key(|m| m.priority);
        for (i, m) in enabled.iter_mut().enumerate() {
            m.priority = i + 1;
        }
    }

    // Build game for symlink removal
    let game = config::GameEntry {
        id: game_id,
        game_type,
        name: game_name,
        path: game_path,
        launch_path: None,
        support_status: cfg.games[game_idx].support_status.clone(),
        mods: cfg.games[game_idx].mods.clone(),
        active_profile_id: None,
        profiles: vec![],
    };
    let mod_entry = game.mods.iter().find(|m| m.id == mod_id).unwrap();

    let deploy_results = match deploy::remove_symlinks(&game, mod_entry) {
        Ok(()) => Some(vec![(mod_entry.name.clone(), true, "Disabled".to_string())]),
        Err(e) => Some(vec![(mod_entry.name.clone(), false, e)]),
    };

    config::save(&cfg).map_err(|e| e.to_string())?;

    Ok(ToggleResult {
        success: true,
        conflict: None,
        deploy_results,
    })
}

#[tauri::command]
fn delete_mod(game_id: String, mod_id: String) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;

    // Remove symlinks if enabled
    if let Some(mod_entry) = game.mods.iter().find(|m| m.id == mod_id) {
        if mod_entry.enabled {
            let _ = deploy::remove_symlinks(game, mod_entry);
        }
    }

    game.mods.retain(|m| m.id != mod_id);
    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

// ═══════════════════════════════════════════════════════
// Batch Operations
// ═══════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchToggleResult {
    pub succeeded: Vec<String>,
    pub failed: Vec<BatchToggleFailure>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchToggleFailure {
    pub mod_id: String,
    pub mod_name: String,
    pub reason: String,
    pub conflicting_mods: Vec<String>,
}

#[tauri::command]
fn batch_toggle(
    game_id: String,
    mod_ids: Vec<String>,
    enabled: bool,
) -> Result<BatchToggleResult, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;

    let game_idx = cfg
        .games
        .iter()
        .position(|g| g.id == game_id)
        .ok_or("Game not found")?;

    let has_load_order = {
        let gt: GameType = serde_json::from_str(&format!("\"{}\"", cfg.games[game_idx].game_type))
            .map_err(|e| format!("Unknown game type: {}", e))?;
        GameRegistry::get(&gt).has_load_order()
    };
    let game_name = cfg.games[game_idx].name.clone();
    let game_type = cfg.games[game_idx].game_type.clone();
    let game_path = cfg.games[game_idx].path.clone();

    if enabled {
        // Build expanded "would-be-enabled" set for conflict checking within the batch
        let mut would_be_enabled: Vec<config::ModEntry> = cfg.games[game_idx]
            .mods
            .iter()
            .filter(|m| m.enabled && !mod_ids.contains(&m.id))
            .cloned()
            .collect();

        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for mod_id in &mod_ids {
            let mod_entry = match cfg.games[game_idx].mods.iter().find(|m| &m.id == mod_id) {
                Some(m) => m.clone(),
                None => {
                    failed.push(BatchToggleFailure {
                        mod_id: mod_id.clone(),
                        mod_name: "unknown".to_string(),
                        reason: "Mod not found".to_string(),
                        conflicting_mods: vec![],
                    });
                    continue;
                }
            };

            if mod_entry.enabled {
                // Already enabled — skip
                succeeded.push(mod_entry.name.clone());
                continue;
            }

            let mut check_entry = mod_entry.clone();
            check_entry.enabled = true;

            let conflict = conflicts::check_enable_conflict(
                &check_entry,
                &would_be_enabled,
                has_load_order,
            );

            if conflict.level == "block" {
                failed.push(BatchToggleFailure {
                    mod_id: mod_entry.id.clone(),
                    mod_name: mod_entry.name.clone(),
                    reason: "Blocked by file conflicts".to_string(),
                    conflicting_mods: conflict.conflicts_with,
                });
                continue;
            }

            // Enable it
            cfg.games[game_idx]
                .mods
                .iter_mut()
                .find(|m| m.id == *mod_id)
                .unwrap()
                .enabled = true;
            would_be_enabled.push(check_entry);
            succeeded.push(mod_entry.name);
        }

        config::save(&cfg).map_err(|e| e.to_string())?;

        // Deploy all enabled mods
        let game = config::GameEntry {
            id: game_id.clone(),
            game_type,
            name: game_name,
            path: game_path,
            launch_path: None,
            support_status: cfg.games[game_idx].support_status.clone(),
            mods: cfg.games[game_idx].mods.clone(),
            active_profile_id: None,
            profiles: vec![],
        };
        let _ = deploy::deploy_mods(&game);

        Ok(BatchToggleResult { succeeded, failed })
    } else {
        // Disabling — simple: set enabled=false, remove symlinks
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for mod_id in &mod_ids {
            let mod_entry = match cfg.games[game_idx].mods.iter().find(|m| &m.id == mod_id) {
                Some(m) => m.clone(),
                None => {
                    failed.push(BatchToggleFailure {
                        mod_id: mod_id.clone(),
                        mod_name: "unknown".to_string(),
                        reason: "Mod not found".to_string(),
                        conflicting_mods: vec![],
                    });
                    continue;
                }
            };

            if !mod_entry.enabled {
                succeeded.push(mod_entry.name);
                continue;
            }

            cfg.games[game_idx]
                .mods
                .iter_mut()
                .find(|m| m.id == *mod_id)
                .unwrap()
                .enabled = false;

            // Remove symlinks
            let game = config::GameEntry {
                id: game_id.clone(),
                game_type: game_type.clone(),
                name: game_name.clone(),
                path: game_path.clone(),
                launch_path: None,
                support_status: cfg.games[game_idx].support_status.clone(),
                mods: cfg.games[game_idx].mods.clone(),
                active_profile_id: None,
                profiles: vec![],
            };
            let _ = deploy::remove_symlinks(&game, &mod_entry);
            succeeded.push(mod_entry.name);
        }

        // Renumber priorities if needed
        if has_load_order {
            let mut enabled: Vec<&mut config::ModEntry> = cfg.games[game_idx]
                .mods
                .iter_mut()
                .filter(|m| m.enabled)
                .collect();
            enabled.sort_by_key(|m| m.priority);
            for (i, m) in enabled.iter_mut().enumerate() {
                m.priority = i + 1;
            }
        }

        config::save(&cfg).map_err(|e| e.to_string())?;

        Ok(BatchToggleResult { succeeded, failed })
    }
}

#[tauri::command]
fn batch_delete(game_id: String, mod_ids: Vec<String>) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;

    for mod_id in &mod_ids {
        if let Some(mod_entry) = game.mods.iter().find(|m| &m.id == mod_id) {
            if mod_entry.enabled {
                let _ = deploy::remove_symlinks(game, mod_entry);
            }
        }
    }

    game.mods.retain(|m| !mod_ids.contains(&m.id));
    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
fn reorder_mods(game_id: String, mod_ids: Vec<String>) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;

    let mut mod_map: HashMap<String, config::ModEntry> =
        game.mods.drain(..).map(|m| (m.id.clone(), m)).collect();
    let mut reordered = Vec::new();
    for (i, id) in mod_ids.iter().enumerate() {
        if let Some(mut m) = mod_map.remove(id) {
            m.priority = i + 1;
            reordered.push(m);
        }
    }
    reordered.extend(mod_map.into_values());
    game.mods = reordered;

    // Re-deploy after reorder (Witcher 3 mods.settings needs new order)
    let _ = deploy::deploy_mods(game);

    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
fn edit_game_path(game_id: String, new_path: String) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;

    let old_path = game.path.clone();
    game.path = new_path;

    // Redeploy all enabled mods to the new path
    let _ = deploy::redeploy_after_path_change(game, &old_path);

    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
fn verify_game_path(game_type: String, path: String) -> Result<bool, String> {
    let gt = GameType::from_str(&game_type);
    deploy::verify_game_path(&gt, &PathBuf::from(&path))?;
    Ok(true)
}

#[tauri::command]
fn check_broken_symlinks(game_id: String) -> Result<Vec<(String, Vec<String>)>, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;
    Ok(deploy::check_all_broken(game))
}

#[tauri::command]
fn check_7z_available() -> Result<String, String> {
    archive::check_7z_available()
}

#[tauri::command]
fn check_conflicts(game_id: String) -> Result<Vec<(String, ConflictInfo)>, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;

    let has_load_order = {
        let gt: GameType = serde_json::from_str(&format!("\"{}\"", game.game_type))
            .map_err(|e| format!("Unknown game type: {}", e))?;
        GameRegistry::get(&gt).has_load_order()
    };
    let results = conflicts::check_all_conflicts(&game.mods, has_load_order);

    Ok(results
        .into_iter()
        .map(|(id, cr)| {
            (
                id,
                ConflictInfo {
                    level: cr.level,
                    conflicts_with: cr.conflicts_with,
                    overlapping_files: cr.overlapping_files,
                },
            )
        })
        .collect())
}

#[tauri::command]
fn deploy_all(game_id: String) -> Result<Vec<(String, bool, String)>, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg.games.iter().find(|g| g.id == game_id).ok_or("Game not found")?;
    deploy::deploy_mods(game)
}

#[tauri::command]
fn purge_all(game_id: String) -> Result<Vec<(String, bool, String)>, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg.games.iter().find(|g| g.id == game_id).ok_or("Game not found")?;
    deploy::remove_all_symlinks(game)
}

#[tauri::command]
fn set_launch_path(game_id: String, launch_path: String) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg.games.iter_mut().find(|g| g.id == game_id).ok_or("Game not found")?;
    game.launch_path = Some(launch_path);
    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
fn launch_game(game_id: String) -> Result<String, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg.games.iter().find(|g| g.id == game_id).ok_or("Game not found")?;
    let launch_path = game.launch_path.as_ref()
        .ok_or("No launch path set. Set it in Edit Game.")?;
    std::process::Command::new(launch_path)
        .current_dir(&game.path)
        .spawn()
        .map_err(|e| format!("Failed to launch: {}", e))?;
    Ok(format!("Launched {}", game.name))
}

// ═══════════════════════════════════════════════════════
// Profile Commands
// ═══════════════════════════════════════════════════════

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    // Use the existing config from the caller (full config serialized from frontend).
    // Frontend sends the AppConfig it already has; save it.
    config::save(&config).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_profile(game_id: String, name: String) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;

    let mod_states: Vec<config::ModState> = game
        .mods
        .iter()
        .map(|m| config::ModState {
            mod_id: m.id.clone(),
            enabled: m.enabled,
            priority: m.priority,
        })
        .collect();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple ISO-ish timestamp
    let ts = format_ts(now);

    let profile = config::Profile {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        game_id: game_id.clone(),
        mod_states,
        created_at: ts,
    };

    game.profiles.push(profile);
    game.active_profile_id = game.profiles.last().map(|p| p.id.clone());
    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
fn delete_profile(game_id: String, profile_id: String) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;

    let was_active = game.active_profile_id.as_deref() == Some(&profile_id);
    game.profiles.retain(|p| p.id != profile_id);
    if was_active {
        game.active_profile_id = None;
    }
    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
fn update_mod_metadata(
    game_id: String,
    mod_id: String,
    name: Option<String>,
    version: Option<String>,
    author: Option<String>,
    description: Option<String>,
    source_url: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;
    let mod_entry = game
        .mods
        .iter_mut()
        .find(|m| m.id == mod_id)
        .ok_or("Mod not found")?;

    if let Some(n) = name { mod_entry.name = n; }
    mod_entry.version = version;
    mod_entry.author = author;
    mod_entry.description = description;
    mod_entry.source_url = source_url;
    mod_entry.category = category;
    if let Some(t) = tags { mod_entry.tags = t; }

    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    mod_entry.updated_at = Some(format_ts(now_ts));

    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
fn rename_profile(game_id: String, profile_id: String, name: String) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;
    let profile = game
        .profiles
        .iter_mut()
        .find(|p| p.id == profile_id)
        .ok_or("Profile not found")?;
    profile.name = name;
    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
fn apply_profile(game_id: String, profile_id: String) -> Result<AppConfig, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;

    let profile = game
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or("Profile not found")?
        .clone();

    // Purge current symlinks
    let _ = deploy::remove_all_symlinks(game);

    // Apply profile states to mods
    for ms in &profile.mod_states {
        if let Some(mod_entry) = game.mods.iter_mut().find(|m| m.id == ms.mod_id) {
            mod_entry.enabled = ms.enabled;
            mod_entry.priority = ms.priority;
        }
    }

    game.active_profile_id = Some(profile_id);

    config::save(&cfg).map_err(|e| e.to_string())?;

    // Redeploy
    let game_ref = cfg.games.iter().find(|g| g.id == game_id).unwrap();
    let _ = deploy::deploy_mods(game_ref);

    Ok(cfg)
}

// ═══════════════════════════════════════════════════════
// Backup / Restore Commands
// ═══════════════════════════════════════════════════════

fn backups_dir() -> PathBuf {
    config_dir().join("backups")
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub filename: String,
    pub timestamp: String,
    pub game_count: usize,
    pub mod_count: usize,
}

#[tauri::command]
fn backup_config() -> Result<BackupInfo, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    let dir = backups_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create backups dir: {}", e))?;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Time error: {}", e))?
        .as_secs();
    let timestamp_str = format_ts(ts);
    let filename = format!("config-{}.json", timestamp_str);
    let dst = dir.join(&filename);

    let data = serde_json::to_string_pretty(&cfg).map_err(|e| format!("Serialization failed: {}", e))?;
    fs::write(&dst, data).map_err(|e| format!("Failed to write backup: {}", e))?;

    let mod_count: usize = cfg.games.iter().map(|g| g.mods.len()).sum();
    Ok(BackupInfo {
        filename,
        timestamp: timestamp_str,
        game_count: cfg.games.len(),
        mod_count,
    })
}

#[tauri::command]
fn list_backups() -> Result<Vec<BackupInfo>, String> {
    let dir = backups_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut backups = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("Failed to read backups dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Read error: {}", e))?;
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            // Extract timestamp from filename (do this before any moves)
            let timestamp = filename
                .strip_prefix("config-")
                .and_then(|s| s.strip_suffix(".json"))
                .unwrap_or("unknown")
                .to_string();

            // Try to parse metadata from backup file
            let (game_count, mod_count) = match fs::read_to_string(&path) {
                Ok(data) => {
                    match serde_json::from_str::<AppConfig>(&data) {
                        Ok(cfg) => (cfg.games.len(), cfg.games.iter().map(|g| g.mods.len()).sum()),
                        Err(_) => (0, 0),
                    }
                }
                Err(_) => (0, 0),
            };

            backups.push(BackupInfo {
                filename,
                timestamp,
                game_count,
                mod_count,
            });
        }
    }

    // Newest first
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(backups)
}

#[tauri::command]
fn restore_config(backup_filename: String) -> Result<AppConfig, String> {
    let src = backups_dir().join(&backup_filename);
    if !src.exists() {
        return Err(format!("Backup '{}' not found.", backup_filename));
    }

    let data = fs::read_to_string(&src)
        .map_err(|e| format!("Failed to read backup: {}", e))?;
    let cfg: AppConfig = serde_json::from_str(&data)
        .map_err(|e| format!("Invalid backup file: {}", e))?;

    // Save as current config
    config::save(&cfg).map_err(|e| e.to_string())?;

    // Redeploy all enabled mods for all games (paths may have changed)
    for game in &cfg.games {
        let _ = deploy::deploy_mods(game);
    }

    Ok(cfg)
}

fn format_ts(seconds: u64) -> String {
    // Manual formatting to avoid chrono dependency
    // Convert to days since epoch, then YYYYMMDD-HHMMSS
    let total_days = seconds / 86400;
    let time_of_day = seconds % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let secs = time_of_day % 60;

    // Approximate year/month/day from days since epoch.
    // This is a simplified algorithm — good enough for filenames.
    let mut year = 1970i64;
    let mut remaining_days = total_days as i64;

    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let month_days = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1usize;
    for &md in &month_days {
        if remaining_days < md as i64 {
            break;
        }
        remaining_days -= md as i64;
        month += 1;
    }
    let day = remaining_days + 1;

    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        year, month, day, hours, minutes, secs
    )
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// ═══════════════════════════════════════════════════════
// Save Scanner Commands
// ═══════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveFile {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified: String,
    pub is_autosave: bool,
    pub is_quicksave: bool,
}

#[tauri::command]
fn scan_saves(game_id: String) -> Result<Vec<SaveFile>, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg
        .games
        .iter()
        .find(|g| g.id == game_id)
        .ok_or("Game not found")?;

    let game_type: GameType = serde_json::from_str(&format!("\"{}\"", game.game_type))
        .map_err(|e| format!("Unknown game type: {}", e))?;
    let registry = GameRegistry::get(&game_type);
    let save_dir = registry.save_dir(&PathBuf::from(&game.path))?;

    if !save_dir.exists() {
        return Err(format!(
            "Save directory not found yet at {} — play and save once first.",
            save_dir.display()
        ));
    }

    let mut saves = Vec::new();
    for entry in fs::read_dir(&save_dir)
        .map_err(|e| format!("Failed to read save dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Read error: {}", e))?;
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        // Filter to known save file extensions — only .sav files, skip .png thumbnails
        let is_save = path.extension()
            .map(|e| {
                let ext = e.to_string_lossy().to_lowercase();
                ext == "sav"
            })
            .unwrap_or(false);

        if !is_save && !name.starts_with("AutoSave") && !name.starts_with("ManualSave") && !name.starts_with("CheckPoint") {
            // Skip non-save files in the directory
            // But include files that seem like saves based on naming
            continue;
        }

        if let Ok(meta) = entry.metadata() {
            let size = meta.len();
            let modified = meta
                .modified()
                .map(|t| {
                    let dur = t.duration_since(UNIX_EPOCH).unwrap_or_default();
                    format_ts(dur.as_secs())
                })
                .unwrap_or_else(|_| "unknown".to_string());

            let is_autosave = name.to_lowercase().contains("auto");
            let is_quicksave = name.to_lowercase().contains("quick");

            saves.push(SaveFile {
                name,
                path: path.to_string_lossy().to_string(),
                size_bytes: size,
                modified,
                is_autosave,
                is_quicksave,
            });
        }
    }

    // Newest first
    saves.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(saves)
}

// ═══════════════════════════════════════════════════════
// Tauri App Entry
// ═══════════════════════════════════════════════════════

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|_app| {
            ensure_dirs().expect("Failed to create config directories");
            db::init().expect("Failed to initialize database");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_game_definitions,
            scan_for_games,
            resolve_dependencies,
            check_updates,
            check_script_extenders,
            scan_plugins,
            validate_plugins,
            export_sync_manifest,
            import_sync_manifest,
            start_download,
            cancel_download,
            get_download_status,
            clear_completed_downloads,
            get_config,
            add_game,
            remove_game,
            import_mod,
            toggle_mod,
            delete_mod,
            batch_toggle,
            batch_delete,
            reorder_mods,
            edit_game_path,
            verify_game_path,
            check_broken_symlinks,
            check_7z_available,
            check_conflicts,
            deploy_all,
            purge_all,
            set_launch_path,
            launch_game,
            save_config,
            update_mod_metadata,
            create_profile,
            delete_profile,
            rename_profile,
            apply_profile,
            backup_config,
            list_backups,
            restore_config,
            scan_saves,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
