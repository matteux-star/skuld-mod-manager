use crate::config::ModEntry;
use std::path::PathBuf;

use super::Game;

pub struct StateOfDecay2;

impl Game for StateOfDecay2 {
    fn name(&self) -> &'static str {
        "State of Decay 2"
    }

    fn mod_path(&self) -> &'static str {
        "Content/Paks/~mods"
    }

    fn valid_mod_formats(&self) -> Vec<&'static str> {
        vec![".pak"]
    }

    fn has_load_order(&self) -> bool {
        false
    }

    fn save_path(&self) -> &'static str {
        // SoD2 saves are deep inside Steam compatdata — path varies per install.
        // Save scanning for SoD2 is not supported yet.
        ""
    }

    fn post_deploy(&self, _game_path: &PathBuf, _mods: &[ModEntry]) -> Result<(), String> {
        // No post-deploy step for SoD2
        Ok(())
    }
}
