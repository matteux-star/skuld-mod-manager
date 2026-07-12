//! Best-effort detection of whether a managed game is currently running.
//! Used to warn the user before deploying/toggling mods (mod files can be locked
//! or the game can pick up a half-deployed load order if it's live).

use sysinfo::System;

use crate::config::GameEntry;

/// Checks running processes for a name or command-line argument matching the game's
/// launch executable (if set) or its install-directory name (covers Proton games,
/// where the OS process is the Windows .exe but the prefix path still contains the
/// install directory name).
pub fn is_game_running(game: &GameEntry) -> bool {
    let needle = match &game.launch_path {
        Some(lp) if !lp.trim().is_empty() => file_stem_lower(lp),
        _ => file_stem_lower(&game.path),
    };
    if needle.is_empty() {
        return false;
    }

    let mut sys = System::new();
    sys.refresh_all();

    sys.processes().values().any(|p| {
        let name = p.name().to_string_lossy().to_lowercase();
        if name.contains(&needle) {
            return true;
        }
        p.cmd()
            .iter()
            .any(|arg| arg.to_string_lossy().to_lowercase().contains(&needle))
    })
}

fn file_stem_lower(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}
