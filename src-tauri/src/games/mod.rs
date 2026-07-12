use crate::config::ModEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

pub mod sod2;
pub mod witcher3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameType {
    #[serde(rename = "witcher3")]
    Witcher3,
    #[serde(rename = "sod2")]
    StateOfDecay2,
    #[serde(rename = "cyberpunk2077")]
    Cyberpunk2077,
    #[serde(rename = "valheim")]
    Valheim,
    #[serde(rename = "stardewvalley")]
    StardewValley,
    #[serde(rename = "kingdomcome")]
    KingdomCome,
    /// Catch-all for unknown/community game types. Stored as the raw string.
    #[serde(untagged)]
    Other(String),
}

impl GameType {
    pub fn as_str(&self) -> &str {
        match self {
            GameType::Witcher3 => "witcher3",
            GameType::StateOfDecay2 => "sod2",
            GameType::Cyberpunk2077 => "cyberpunk2077",
            GameType::Valheim => "valheim",
            GameType::StardewValley => "stardewvalley",
            GameType::KingdomCome => "kingdomcome",
            GameType::Other(s) => s.as_str(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "witcher3" => GameType::Witcher3,
            "sod2" => GameType::StateOfDecay2,
            "cyberpunk2077" => GameType::Cyberpunk2077,
            "valheim" => GameType::Valheim,
            "stardewvalley" => GameType::StardewValley,
            "kingdomcome" => GameType::KingdomCome,
            other => GameType::Other(other.to_string()),
        }
    }
}

// ═══════════════════════════════════════════════════════
// Declarative Game Definitions (JSON-backed)
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDefinition {
    #[serde(rename = "type")]
    pub game_type: String,
    pub display_name: String,
    pub engine: String,
    pub icon: String,
    pub support_status: String,
    pub mod_directory: PathSpec,
    pub save_directory: PathSpec,
    #[serde(default)]
    pub save_pattern: Option<String>,
    pub validation: ValidationSpec,
    pub install_files: InstallFilesSpec,
    pub load_order: LoadOrderSpec,
    pub conflicts: String,
    pub launch: LaunchSpec,
    pub detection: DetectionSpec,
    pub proton: Option<ProtonSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathSpec {
    pub relative_to: String, // "installPath" | "protonPrefix"
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationSpec {
    pub mode: String,  // "containsSubdirectory" | "containsFileType"
    pub value: String, // subdirectory name or file extension
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallFilesSpec {
    pub mode: String,         // "relativePaths" | "flatFiles"
    #[serde(default)]
    pub filter: Option<String>, // file extension filter for flatFiles mode
    #[serde(default)]
    pub strip_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadOrderSpec {
    pub supported: bool,
    pub config_file: Option<ConfigFileSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFileSpec {
    pub relative_to: String,
    pub path: String,
    pub format: String,   // "iniSection"
    pub section: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSpec {
    pub steam_app_id: Option<u32>,
    pub known_executables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionSpec {
    pub steam: Option<SteamDetection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamDetection {
    pub app_id: u32,
    pub dir_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtonSpec {
    pub app_id: u32,
    pub relative_exe_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptExtenderSpec {
    pub name: String,
    pub short_name: String,
    pub executable_name: String,
    pub required_files: Vec<String>,
    pub website: String,
    pub is_launcher: bool,
}

// ═══════════════════════════════════════════════════════
// Script Extender Registry
// ═══════════════════════════════════════════════════════

pub fn get_script_extenders() -> Vec<(&'static str, ScriptExtenderSpec)> {
    vec![
        ("skyrimse", ScriptExtenderSpec {
            name: "Skyrim Script Extender".into(),
            short_name: "SKSE".into(),
            executable_name: "skse64_loader.exe".into(),
            required_files: vec!["skse64_1_6_640.dll".into(), "skse64_steam_loader.dll".into()],
            website: "https://skse.silverlock.org/".into(),
            is_launcher: true,
        }),
        ("witcher3", ScriptExtenderSpec {
            name: "REDmod".into(),
            short_name: "REDmod".into(),
            executable_name: "REDmod.exe".into(),
            required_files: vec!["tools/redmod/bin/redmod.exe".into()],
            website: "https://www.cyberpunk.net/us/modding-support".into(),
            is_launcher: false,
        }),
    ]
}

/// Cached game definitions loaded at startup.
static DEFINITIONS: OnceLock<HashMap<String, GameDefinition>> = OnceLock::new();

pub fn load_definitions() -> &'static HashMap<String, GameDefinition> {
    DEFINITIONS.get_or_init(|| {
        let mut map = HashMap::new();
        let builtin = [
            include_str!("../../games/definitions/witcher3.json"),
            include_str!("../../games/definitions/sod2.json"),
            include_str!("../../games/definitions/cyberpunk2077.json"),
            include_str!("../../games/definitions/valheim.json"),
            include_str!("../../games/definitions/stardewvalley.json"),
            include_str!("../../games/definitions/kingdomcome.json"),
        ];
        for json in builtin {
            if let Ok(def) = serde_json::from_str::<GameDefinition>(json) {
                map.insert(def.game_type.clone(), def);
            }
        }
        map
    })
}

pub fn get_definition(game_type: &str) -> Option<&'static GameDefinition> {
    load_definitions().get(game_type)
}

pub fn all_definitions() -> Vec<&'static GameDefinition> {
    load_definitions().values().collect()
}

// ═══════════════════════════════════════════════════════
// Legacy Game Trait (custom hooks for deploy behaviour)
// ═══════════════════════════════════════════════════════

/// Defines behaviour shared by every supported game.
pub trait Game {
    /// Human-readable name.
    fn name(&self) -> &'static str;
    /// Resolve the directory mods are deployed into.
    fn mod_dir(&self, game_path: &Path) -> Result<PathBuf, String>;
    /// File extensions or folder patterns that make a mod valid.
    fn valid_mod_formats(&self) -> Vec<&'static str>;
    /// Whether load order matters for this game.
    fn has_load_order(&self) -> bool;
    /// Resolve the directory save files live in.
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
            // Generic fallback for any other game type: uses definition-based behaviour
            _ => Box::new(GenericGame { game_type: game_type.as_str().to_string() }),
        }
    }
}

/// Generic game implementation for any game type not having a custom Rust hook.
/// Behaviour is driven entirely by the JSON definition.
struct GenericGame {
    game_type: String,
}

impl Game for GenericGame {
    fn name(&self) -> &'static str {
        // Leak the string to satisfy 'static lifetime — acceptable since Game
        // instances are short-lived and this is called rarely.
        // Actually, we need the definition to provide the name. Let's return a
        // reasonable default.
        "Unknown Game"
    }

    fn mod_dir(&self, game_path: &Path) -> Result<PathBuf, String> {
        // Resolve from definition
        let def = get_definition(&self.game_type)
            .ok_or_else(|| format!("No definition for game type: {}", self.game_type))?;
        match def.mod_directory.relative_to.as_str() {
            "installPath" => Ok(game_path.join(&def.mod_directory.path)),
            other => Err(format!("Unknown relativeTo: {}", other)),
        }
    }

    fn valid_mod_formats(&self) -> Vec<&'static str> {
        vec!["*"]
    }

    fn has_load_order(&self) -> bool {
        get_definition(&self.game_type)
            .map(|d| d.load_order.supported)
            .unwrap_or(false)
    }

    fn save_dir(&self, game_path: &Path) -> Result<PathBuf, String> {
        let def = get_definition(&self.game_type)
            .ok_or_else(|| format!("No definition for game type: {}", self.game_type))?;
        Ok(game_path.join(&def.save_directory.path))
    }

    fn support_status(&self) -> &'static str {
        "provisional"
    }
}
