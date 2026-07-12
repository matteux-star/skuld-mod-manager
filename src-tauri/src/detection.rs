use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Result of scanning for an installed game.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedGame {
    pub game_type: String,
    pub display_name: String,
    pub install_path: String,
    pub source: String, // "steam" | "lutris" | "heroic"
    pub source_detail: String,
}

/// Scan Steam libraries for known games.
pub fn scan_steam() -> Vec<DetectedGame> {
    let mut found = Vec::new();

    // Find Steam root
    let steam_home = dirs::home_dir()
        .map(|h| h.join(".steam").join("steam"))
        .filter(|p| p.exists());

    let steam_home = match steam_home {
        Some(p) => p,
        None => {
            // Try Flatpak path
            let flatpak = dirs::home_dir()
                .map(|h| h.join(".var").join("app").join("com.valvesoftware.Steam").join(".steam").join("steam"))
                .filter(|p| p.exists());
            match flatpak {
                Some(p) => p,
                None => return found,
            }
        }
    };

    // Collect all library paths
    let mut lib_dirs = vec![steam_home.clone()]; // default library

    let lib_folders_vdf = steam_home.join("steamapps").join("libraryfolders.vdf");
    if let Ok(content) = fs::read_to_string(&lib_folders_vdf) {
        // Simple VDF parser: extract "path" values
        for line in content.lines() {
            let line = line.trim();
            if let Some(path) = parse_vdf_path(line) {
                let lib = PathBuf::from(&path).join("steamapps").join("common");
                if lib.exists() {
                    lib_dirs.push(PathBuf::from(&path));
                }
            }
        }
    }

    // Known games registry (from game definitions)
    let known: Vec<(&str, &str, u32, Vec<&str>, bool)> = vec![
        ("witcher3", "The Witcher 3: Wild Hunt", 292030, vec!["The Witcher 3", "The Witcher 3 Wild Hunt"], false),
        ("sod2", "State of Decay 2", 495420, vec!["State of Decay 2", "StateOfDecay2"], true),
    ];

    for lib_path in &lib_dirs {
        let common = lib_path.join("steamapps").join("common");
        if !common.exists() { continue; }

        for (game_type, display_name, _app_id, dir_names, _is_proton) in &known {
            for dir_name in dir_names {
                let install_path = common.join(dir_name);
                if install_path.exists() && install_path.is_dir() {
                    // Verify: check for known executable or marker
                    let verified = verify_install(*game_type, &install_path);
                    if verified {
                        found.push(DetectedGame {
                            game_type: game_type.to_string(),
                            display_name: display_name.to_string(),
                            install_path: install_path.to_string_lossy().to_string(),
                            source: "steam".to_string(),
                            source_detail: format!("Steam Library ({})", lib_path.display()),
                        });
                        break; // found this game, skip other dir names
                    }
                }
            }
        }
    }

    found
}

fn parse_vdf_path(line: &str) -> Option<String> {
    // VDF format: "path"\t\t"/mnt/games/SteamLibrary"
    let line = line.trim();
    if !line.starts_with("\"path\"") {
        return None;
    }
    // Find the last quoted string on the line
    let after_key = line.strip_prefix("\"path\"")?.trim_start();
    // Skip tabs/spaces, then the value is in quotes
    let value = after_key.trim_start_matches(&['\t', ' '][..]).trim_matches('"');
    if !value.is_empty() {
        return Some(value.to_string());
    }
    None
}

fn verify_install(game_type: &str, install_path: &Path) -> bool {
    match game_type {
        "witcher3" => {
            install_path.join("bin").join("x64").join("witcher3.exe").exists()
                || install_path.join("bin").join("x64_vk").join("witcher3.exe").exists()
        }
        "sod2" => {
            install_path.join("StateOfDecay2.exe").exists()
                || install_path.join("Binaries").join("Win64").join("StateOfDecay2-Win64-Shipping.exe").exists()
        }
        _ => install_path.is_dir(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vdf_path() {
        assert_eq!(
            parse_vdf_path("\"path\"\t\t\"/mnt/games/SteamLibrary\""),
            Some("/mnt/games/SteamLibrary".to_string())
        );
        assert_eq!(parse_vdf_path("\"something\"\t\"value\""), None);
    }
}
