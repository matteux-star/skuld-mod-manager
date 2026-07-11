use crate::config::ModEntry;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub mod sod2;
pub mod witcher3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameType {
    #[serde(rename = "witcher3")]
    Witcher3,
    #[serde(rename = "sod2")]
    StateOfDecay2,
}

/// Defines behaviour shared by every supported game.
pub trait Game {
    /// Human-readable name.
    fn name(&self) -> &'static str;
    /// Resolve the directory mods are deployed into. May live outside the
    /// game install root (e.g. inside a Proton prefix). The error string is
    /// shown directly in the UI.
    fn mod_dir(&self, game_path: &Path) -> Result<PathBuf, String>;
    /// File extensions or folder patterns that make a mod valid.
    fn valid_mod_formats(&self) -> Vec<&'static str>;
    /// Whether load order matters for this game.
    fn has_load_order(&self) -> bool;
    /// Resolve the directory save files live in. The error string is shown
    /// directly in the UI.
    fn save_dir(&self, game_path: &Path) -> Result<PathBuf, String>;
    /// Support status assigned when the game is added.
    fn support_status(&self) -> &'static str;
    /// Post-deploy step, if any (e.g. regenerate mods.settings).
    fn post_deploy(&self, _game_path: &PathBuf, _mods: &[ModEntry]) -> Result<(), String> {
        Ok(())
    }
}

/// Statically-known game registry.
pub struct GameRegistry;

impl GameRegistry {
    pub fn get(game_type: &GameType) -> Box<dyn Game> {
        match game_type {
            GameType::Witcher3 => Box::new(witcher3::Witcher3),
            GameType::StateOfDecay2 => Box::new(sod2::StateOfDecay2),
        }
    }

    pub fn all_types() -> Vec<GameType> {
        vec![GameType::Witcher3, GameType::StateOfDecay2]
    }
}
