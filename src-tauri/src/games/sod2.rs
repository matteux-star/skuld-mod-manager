use std::path::{Path, PathBuf};

use super::Game;

/// Steam App ID for State of Decay 2: Juggernaut Edition.
const SOD2_APP_ID: &str = "495420";

pub struct StateOfDecay2;

/// Resolve the `.../StateOfDecay2/Saved` directory inside the game's Proton
/// prefix, starting from the user-supplied install path
/// (`<library>/steamapps/common/StateOfDecay2`).
///
/// SoD2 does not read mods from its install directory — paks go in
/// `AppData/Local/StateOfDecay2/Saved/Paks` inside the compatdata prefix.
fn resolve_proton_saved_dir(game_path: &Path) -> Result<PathBuf, String> {
    let steamapps = game_path
        .ancestors()
        .find(|p| p.file_name().map(|n| n == "steamapps").unwrap_or(false))
        .ok_or_else(|| format!(
            "Game path doesn't look like a Steam install (no 'steamapps' in {}). \
             Expected something like .../steamapps/common/StateOfDecay2.",
            game_path.display()
        ))?;

    let prefix = steamapps
        .join("compatdata")
        .join(SOD2_APP_ID)
        .join("pfx");

    if !prefix.is_dir() {
        return Err(format!(
            "Proton prefix not found at {}. Launch the game once through Steam/Proton, then retry.",
            prefix.display()
        ));
    }

    Ok(prefix
        .join("drive_c")
        .join("users")
        .join("steamuser")
        .join("AppData")
        .join("Local")
        .join("StateOfDecay2")
        .join("Saved"))
}

impl Game for StateOfDecay2 {
    fn name(&self) -> &'static str {
        "State of Decay 2"
    }

    fn mod_dir(&self, game_path: &Path) -> Result<PathBuf, String> {
        Ok(resolve_proton_saved_dir(game_path)?.join("Paks"))
    }

    fn valid_mod_formats(&self) -> Vec<&'static str> {
        vec![".pak"]
    }

    fn has_load_order(&self) -> bool {
        false
    }

    fn save_dir(&self, game_path: &Path) -> Result<PathBuf, String> {
        Ok(resolve_proton_saved_dir(game_path)?.join("SaveGames"))
    }

    fn support_status(&self) -> &'static str {
        "provisional"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_steam_install(with_prefix: bool) -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let game_path = root
            .path()
            .join("steamapps")
            .join("common")
            .join("StateOfDecay2");
        std::fs::create_dir_all(&game_path).unwrap();
        if with_prefix {
            let saved = root
                .path()
                .join("steamapps")
                .join("compatdata")
                .join(SOD2_APP_ID)
                .join("pfx")
                .join("drive_c")
                .join("users")
                .join("steamuser")
                .join("AppData")
                .join("Local")
                .join("StateOfDecay2")
                .join("Saved");
            std::fs::create_dir_all(&saved).unwrap();
        }
        (root, game_path)
    }

    #[test]
    fn resolves_mod_and_save_dirs_from_steam_library() {
        let (root, game_path) = fake_steam_install(true);
        let saved = root
            .path()
            .join("steamapps/compatdata/495420/pfx/drive_c/users/steamuser/AppData/Local/StateOfDecay2/Saved");

        assert_eq!(StateOfDecay2.mod_dir(&game_path).unwrap(), saved.join("Paks"));
        assert_eq!(StateOfDecay2.save_dir(&game_path).unwrap(), saved.join("SaveGames"));
    }

    #[test]
    fn errors_when_path_is_not_a_steam_library() {
        let root = tempfile::tempdir().unwrap();
        let game_path = root.path().join("Games").join("StateOfDecay2");
        std::fs::create_dir_all(&game_path).unwrap();

        let err = StateOfDecay2.mod_dir(&game_path).unwrap_err();
        assert!(err.contains("steamapps"), "error should mention steamapps: {}", err);
    }

    #[test]
    fn errors_when_proton_prefix_missing() {
        let (_root, game_path) = fake_steam_install(false);

        let err = StateOfDecay2.mod_dir(&game_path).unwrap_err();
        assert!(err.contains("compatdata"), "error should name expected path: {}", err);
        assert!(err.contains("Proton"), "error should mention Proton: {}", err);
    }
}
