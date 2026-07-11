use std::collections::HashSet;

use crate::config::ModEntry;

/// Conflict check result for a single mod.
#[derive(Debug, Clone)]
pub struct ConflictResult {
    /// Whether the mod can be enabled.
    pub allowed: bool,
    /// "warn" (Witcher 3 — allowed but warned), "block" (SoD2 — not allowed), or "none"
    pub level: String,
    /// Names of mods this mod conflicts with.
    pub conflicts_with: Vec<String>,
    /// The overlapping file paths.
    pub overlapping_files: Vec<String>,
}

/// Check if enabling `target_mod` would conflict with any already-enabled mods
/// for the given game. `has_load_order` determines blocking vs warning behaviour.
pub fn check_enable_conflict(
    target_mod: &ModEntry,
    enabled_mods: &[ModEntry],
    has_load_order: bool,
) -> ConflictResult {
    if !target_mod.enabled {
        // Mod is being disabled — no conflict to check
        return ConflictResult {
            allowed: true,
            level: "none".to_string(),
            conflicts_with: vec![],
            overlapping_files: vec![],
        };
    }

    let target_files: HashSet<&str> = target_mod.installed_files.iter().map(|s| s.as_str()).collect();
    let mut conflicts_with = Vec::new();
    let mut overlapping_files = Vec::new();

    for other in enabled_mods {
        if other.id == target_mod.id || !other.enabled {
            continue;
        }

        for file in &other.installed_files {
            if target_files.contains(file.as_str()) {
                if !conflicts_with.contains(&other.name) {
                    conflicts_with.push(other.name.clone());
                }
                if !overlapping_files.contains(file) {
                    overlapping_files.push(file.clone());
                }
            }
        }
    }

    if conflicts_with.is_empty() {
        ConflictResult {
            allowed: true,
            level: "none".to_string(),
            conflicts_with: vec![],
            overlapping_files: vec![],
        }
    } else if has_load_order {
        ConflictResult {
            allowed: true,
            level: "warn".to_string(),
            conflicts_with,
            overlapping_files,
        }
    } else {
        ConflictResult {
            allowed: false,
            level: "block".to_string(),
            conflicts_with,
            overlapping_files,
        }
    }
}

/// Check all mods for a game and return results.
/// Re-run this after priority changes for Witcher 3.
pub fn check_all_conflicts(
    mods: &[ModEntry],
    has_load_order: bool,
) -> Vec<(String, ConflictResult)> {
    let enabled: Vec<&ModEntry> = mods.iter().filter(|m| m.enabled).collect();

    mods.iter()
        .map(|m| {
            // Clone enabled mods into owned Vec
            let owned_enabled: Vec<ModEntry> = enabled.iter().map(|&e| e.clone()).collect();
            let result = check_enable_conflict(m, &owned_enabled, has_load_order);
            (m.id.clone(), result)
        })
        .collect()
}
