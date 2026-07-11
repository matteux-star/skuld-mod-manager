use std::fs;
use std::io;
use std::os::unix;
use std::path::{Path, PathBuf};

use crate::config::{GameEntry, ModEntry};
use crate::games::{GameRegistry, GameType};

/// Create a symlink from `target` (in the game's mod dir) to `source` (in the library).
fn create_symlink(source: &Path, target: &Path) -> Result<(), String> {
    // Ensure parent dir exists
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create mod dir {}: {}", parent.display(), e))?;
    }

    // Remove existing symlink or file at target
    if target.exists() || target.is_symlink() {
        fs::remove_file(target)
            .or_else(|_| fs::remove_dir_all(target))
            .map_err(|e| format!("Failed to remove existing target {}: {}", target.display(), e))?;
    }

    unix::fs::symlink(source, target)
        .map_err(|e| format!("Failed to create symlink {} -> {}: {}", target.display(), source.display(), e))
}

/// Deploy all enabled mods for a game as symlinks.
/// Returns per-mod results: Vec<(mod_name, success, message)>
pub fn deploy_mods(game: &GameEntry) -> Result<Vec<(String, bool, String)>, String> {
    deploy_mods_from_library(game, &crate::library_dir())
}

fn deploy_mods_from_library(
    game: &GameEntry,
    library_root: &Path,
) -> Result<Vec<(String, bool, String)>, String> {
    let game_type: GameType = serde_json::from_str(&format!("\"{}\"", game.game_type))
        .map_err(|e| format!("Unknown game type: {}", e))?;
    let registry = GameRegistry::get(&game_type);
    let game_path = PathBuf::from(&game.path);
    let mod_base = registry.mod_dir(&game_path)?;

    let mut results = Vec::new();

    for m in &game.mods {
        if !m.enabled {
            continue;
        }

        let mod_root = library_root
            .join(&game.game_type)
            .join(&m.name);

        if !mod_root.exists() {
            results.push((m.name.clone(), false, "Library files not found. Try re-importing.".to_string()));
            continue;
        }

        let mut success = true;
        let mut errors = Vec::new();

        for file_path in &m.installed_files {
            let source = mod_root.join(file_path);
            let target = mod_base.join(file_path);

            if let Err(e) = create_symlink(&source, &target) {
                success = false;
                errors.push(e);
            }
        }

        if success {
            results.push((m.name.clone(), true, "Deployed".to_string()));
        } else {
            results.push((m.name.clone(), false, errors.join("; ")));
        }
    }

    // Run post-deploy (e.g., mods.settings for Witcher 3)
    if let Err(e) = registry.post_deploy(&game_path, &game.mods) {
        results.push(("post-deploy".to_string(), false, e));
    }

    Ok(results)
}

/// Remove symlinks for a specific mod from the game's mod directory.
/// Does not fail if symlinks are already gone.
pub fn remove_symlinks(game: &GameEntry, mod_entry: &ModEntry) -> Result<(), String> {
    let game_type: GameType = serde_json::from_str(&format!("\"{}\"", game.game_type))
        .map_err(|e| format!("Unknown game type: {}", e))?;
    let registry = GameRegistry::get(&game_type);
    let mod_base = registry.mod_dir(&PathBuf::from(&game.path))?;

    for file_path in &mod_entry.installed_files {
        let target = mod_base.join(file_path);
        if target.is_symlink() || target.exists() {
            if let Err(e) = fs::remove_file(&target) {
                // Don't fail if file is already gone
                if e.kind() != io::ErrorKind::NotFound {
                    return Err(format!("Failed to remove {}: {}", target.display(), e));
                }
            }
        }
    }

    Ok(())
}

/// Remove all symlinks for all enabled mods of a game.
pub fn remove_all_symlinks(game: &GameEntry) -> Result<Vec<(String, bool, String)>, String> {
    let mut results = Vec::new();
    for m in &game.mods {
        if m.enabled {
            match remove_symlinks(game, m) {
                Ok(()) => results.push((m.name.clone(), true, "Removed".to_string())),
                Err(e) => results.push((m.name.clone(), false, e)),
            }
        }
    }
    Ok(results)
}

/// Check if a mod's symlinks are still valid (target still exists).
/// Returns a list of broken symlink paths.
pub fn check_broken_symlinks(game: &GameEntry, mod_entry: &ModEntry) -> Vec<String> {
    let game_type: GameType = match serde_json::from_str(&format!("\"{}\"", game.game_type)) {
        Ok(gt) => gt,
        Err(_) => return vec![],
    };
    let registry = GameRegistry::get(&game_type);
    let mod_base = match registry.mod_dir(&PathBuf::from(&game.path)) {
        Ok(dir) => dir,
        Err(_) => return vec![],
    };

    let mut broken = Vec::new();

    for file_path in &mod_entry.installed_files {
        let target = mod_base.join(file_path);
        if target.is_symlink() {
            match fs::read_link(&target) {
                Ok(source) => {
                    if !source.exists() {
                        broken.push(file_path.clone());
                    }
                }
                Err(_) => {
                    broken.push(file_path.clone());
                }
            }
        }
    }

    broken
}

/// Check all enabled mods for a game for broken symlinks.
/// Returns Vec<(mod_name, broken_paths)>.
pub fn check_all_broken(game: &GameEntry) -> Vec<(String, Vec<String>)> {
    game.mods
        .iter()
        .filter(|m| m.enabled)
        .map(|m| {
            let broken = check_broken_symlinks(game, m);
            (m.name.clone(), broken)
        })
        .filter(|(_, broken)| !broken.is_empty())
        .collect()
}

/// Redeploy all enabled mods after a game path change.
/// Removes old symlinks, creates new ones, reports per-mod.
pub fn redeploy_after_path_change(
    game: &GameEntry,
    old_path: &str,
) -> Result<Vec<(String, bool, String)>, String> {
    let mut old_game = game.clone();
    old_game.path = old_path.to_string();

    // Remove old symlinks (don't fail if path is gone)
    let _ = remove_all_symlinks(&old_game);

    // Deploy to new path
    deploy_mods(game)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GameEntry, ModEntry};

    #[test]
    fn sod2_deploy_symlinks_paks_into_proton_prefix_without_doubled_paths() {
        let root = tempfile::tempdir().unwrap();

        // Fake Steam library with Proton prefix
        let game_path = root.path().join("steamapps/common/StateOfDecay2");
        let saved = root.path().join(
            "steamapps/compatdata/495420/pfx/drive_c/users/steamuser/AppData/Local/StateOfDecay2/Saved",
        );
        std::fs::create_dir_all(&game_path).unwrap();
        std::fs::create_dir_all(&saved).unwrap();

        // Fake mod library with one pak
        let library = root.path().join("library");
        let mod_root = library.join("sod2/CoolMod");
        std::fs::create_dir_all(&mod_root).unwrap();
        std::fs::write(mod_root.join("cool.pak"), b"pak").unwrap();

        let game = GameEntry {
            id: "g".into(),
            game_type: "sod2".into(),
            name: "SoD2".into(),
            path: game_path.to_string_lossy().to_string(),
            launch_path: None,
            support_status: "provisional".into(),
            mods: vec![ModEntry {
                id: "m".into(),
                name: "CoolMod".into(),
                archive_source: "cool.zip".into(),
                enabled: true,
                priority: 1,
                installed_files: vec!["cool.pak".into()],
            }],
        };

        let results = deploy_mods_from_library(&game, &library).unwrap();
        assert!(results.iter().all(|(_, ok, _)| *ok), "deploy failed: {:?}", results);

        let target = saved.join("Paks/cool.pak");
        assert!(target.is_symlink(), "expected symlink at {}", target.display());
        assert_eq!(std::fs::read_link(&target).unwrap(), mod_root.join("cool.pak"));

        // Removal cleans the symlink
        remove_symlinks(&game, &game.mods[0]).unwrap();
        assert!(!target.exists() && !target.is_symlink());
    }
}

/// Verify that the game directory looks valid before registering.
pub fn verify_game_path(game_type: &GameType, path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err("Directory does not exist.".to_string());
    }
    if !path.is_dir() {
        return Err("Path is not a directory.".to_string());
    }

    let registry = GameRegistry::get(game_type);
    let mod_path = registry.mod_dir(path)?;

    // Check if mod path exists or can be created
    if !mod_path.exists() {
        // Try to create it to see if we have write permission
        fs::create_dir_all(&mod_path)
            .map_err(|e| format!(
                "Cannot create mod directory at {}. Check permissions: {}",
                mod_path.display(), e
            ))?;
    }

    Ok(())
}
