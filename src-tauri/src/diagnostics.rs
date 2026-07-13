//! Read-only health checks for a game's mod-loading pipeline, plus recovery
//! for library folders that were extracted but lost their DB-side mod entry
//! (e.g. after `remove_game` cascade-deletes `mods` rows while leaving the
//! extracted archive on disk).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::GameEntry;
use crate::games::{GameRegistry, GameType};
use crate::{deploy, library_dir};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub findings: Vec<Finding>,
    /// Library folder names present on disk with no matching `mods` row —
    /// candidates for `adopt_orphaned_mod`.
    pub orphaned_folders: Vec<String>,
}

fn info(message: impl Into<String>) -> Finding {
    Finding { severity: Severity::Info, message: message.into() }
}
fn warning(message: impl Into<String>) -> Finding {
    Finding { severity: Severity::Warning, message: message.into() }
}
fn error(message: impl Into<String>) -> Finding {
    Finding { severity: Severity::Error, message: message.into() }
}

pub fn diagnose_game(game: &GameEntry) -> DiagnosticReport {
    diagnose_game_with_library_root(game, &library_dir())
}

fn diagnose_game_with_library_root(game: &GameEntry, library_root: &std::path::Path) -> DiagnosticReport {
    let mut findings = Vec::new();
    let game_path = PathBuf::from(&game.path);

    // 1. Install path.
    if !game_path.is_dir() {
        findings.push(error(format!(
            "Install path does not exist: {}",
            game_path.display()
        )));
    } else {
        findings.push(info(format!("Install path OK: {}", game_path.display())));
    }

    // 2. mod_dir / save_dir resolution (Proton prefix for SoD2, etc).
    let gt = GameType::from_str(&game.game_type);
    let registry = GameRegistry::get(&gt);
    match registry.mod_dir(&game_path) {
        Ok(mod_dir) => {
            if mod_dir.is_dir() {
                findings.push(info(format!("Mod directory resolved: {}", mod_dir.display())));
            } else {
                findings.push(warning(format!(
                    "Mod directory resolved but doesn't exist yet: {}",
                    mod_dir.display()
                )));
            }
        }
        Err(e) => findings.push(error(format!("Could not resolve mod directory: {}", e))),
    }

    // 3. Orphaned library folders: extracted on disk, no mods row.
    let game_library_dir = library_root.join(&game.game_type);
    let mut orphaned_folders = Vec::new();
    if let Ok(entries) = fs::read_dir(&game_library_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if !entry.path().is_dir() {
                continue;
            }
            let folder_name = entry.file_name().to_string_lossy().to_string();
            let has_mod = game.mods.iter().any(|m| m.name == folder_name);
            if !has_mod {
                orphaned_folders.push(folder_name);
            }
        }
    }
    if !orphaned_folders.is_empty() {
        findings.push(warning(format!(
            "{} extracted mod folder(s) have no matching entry in the mod list (orphaned by a previous remove/re-add or a failed import): {}",
            orphaned_folders.len(),
            orphaned_folders.join(", ")
        )));
    }

    // 4. Broken symlinks for currently-enabled mods.
    let broken = deploy::check_all_broken(game);
    if !broken.is_empty() {
        for (mod_name, paths) in &broken {
            findings.push(error(format!(
                "{}: broken symlink(s): {}",
                mod_name,
                paths.join(", ")
            )));
        }
    }

    // 5. installed_files vs actual files present in the mod's library folder.
    for m in &game.mods {
        let mod_root = library_root.join(&game.game_type).join(&m.name);
        if !mod_root.is_dir() {
            findings.push(error(format!(
                "{}: library folder is missing ({})",
                m.name,
                mod_root.display()
            )));
            continue;
        }
        for file_path in &m.installed_files {
            if !mod_root.join(file_path).exists() {
                findings.push(error(format!(
                    "{}: installed file missing from library: {}",
                    m.name, file_path
                )));
            }
        }
    }

    if findings.iter().all(|f| matches!(f.severity, Severity::Info)) {
        findings.push(info("No problems found."));
    }

    DiagnosticReport { findings, orphaned_folders }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GameEntry;

    fn fake_sod2_install() -> (tempfile::TempDir, GameEntry) {
        let root = tempfile::tempdir().unwrap();
        let game_path = root.path().join("steamapps/common/StateOfDecay2");
        let saved = root.path().join(
            "steamapps/compatdata/495420/pfx/drive_c/users/steamuser/AppData/Local/StateOfDecay2/Saved",
        );
        std::fs::create_dir_all(&game_path).unwrap();
        std::fs::create_dir_all(&saved).unwrap();

        let game = GameEntry {
            id: "g".into(),
            game_type: "sod2".into(),
            name: "SoD2".into(),
            path: game_path.to_string_lossy().to_string(),
            launch_path: None,
            support_status: "provisional".into(),
            active_profile_id: None,
            profiles: vec![],
            mods: vec![],
        };
        (root, game)
    }

    #[test]
    fn flags_orphaned_library_folder_with_no_mods_row() {
        let (root, game) = fake_sod2_install();

        // Simulate a library folder left behind after remove_game
        // cascade-deleted the mods row: the extracted archive is still on
        // disk, but `game.mods` is empty.
        let lib_dir = root.path().join("library/sod2/OrphanedMod");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::write(lib_dir.join("orphan_P.pak"), b"pak").unwrap();

        let report = diagnose_game_with_library_root(&game, &root.path().join("library"));

        assert_eq!(report.orphaned_folders, vec!["OrphanedMod".to_string()]);
        assert!(report
            .findings
            .iter()
            .any(|f| f.message.contains("OrphanedMod")));
    }

    #[test]
    fn does_not_flag_folder_with_matching_mod_row() {
        let (root, mut game) = fake_sod2_install();
        let lib_dir = root.path().join("library/sod2/KnownMod");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::write(lib_dir.join("known_P.pak"), b"pak").unwrap();

        game.mods.push(crate::config::ModEntry {
            id: "m".into(),
            name: "KnownMod".into(),
            archive_source: "known.zip".into(),
            enabled: false,
            priority: 1,
            installed_files: vec!["known_P.pak".into()],
            version: None,
            author: None,
            description: None,
            source_url: None,
            category: None,
            tags: vec![],
            installed_at: None,
            updated_at: None,
            relationships: vec![],
        });

        let report = diagnose_game_with_library_root(&game, &root.path().join("library"));
        assert!(report.orphaned_folders.is_empty());
    }
}
