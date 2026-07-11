use crate::config::ModEntry;
use std::path::PathBuf;

use super::Game;

pub struct Witcher3;

impl Game for Witcher3 {
    fn name(&self) -> &'static str {
        "The Witcher 3"
    }

    fn mod_path(&self) -> &'static str {
        "Mods"
    }

    fn valid_mod_formats(&self) -> Vec<&'static str> {
        vec!["content/"]
    }

    fn has_load_order(&self) -> bool {
        true
    }

    fn save_path(&self) -> &'static str {
        "Documents/The Witcher 3/gamesaves"
    }

    fn post_deploy(&self, game_path: &PathBuf, mods: &[ModEntry]) -> Result<(), String> {
        // Regenerate mods.settings with priority order
        let settings_path = game_path
            .join("Documents")
            .join("The Witcher 3")
            .join("mods.settings");

        if let Some(parent) = settings_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create settings dir: {}", e))?;
        }

        let mut content = String::from("[mods]\n");
        let mut enabled_mods: Vec<&ModEntry> = mods.iter()
            .filter(|m| m.enabled)
            .collect();
        enabled_mods.sort_by_key(|m| m.priority);

        for m in &enabled_mods {
            content.push_str(&format!(
                "Enabled={}\n",
                m.name
            ));
            content.push_str(&format!(
                "Priority={}\n",
                m.priority
            ));
        }

        std::fs::write(&settings_path, content)
            .map_err(|e| format!("Failed to write mods.settings: {}", e))?;

        Ok(())
    }
}
