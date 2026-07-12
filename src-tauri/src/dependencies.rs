use crate::config::{GameEntry, ModEntry, ModRelationEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyResult {
    pub can_enable: bool,
    pub requires_enable: Vec<RequiredMod>,
    pub must_disable: Vec<ConflictingMod>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredMod {
    pub mod_id: Option<String>,
    pub mod_name: String,
    pub is_installed: bool,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictingMod {
    pub mod_id: Option<String>,
    pub mod_name: String,
    pub is_installed: bool,
    pub is_enabled: bool,
}

/// Resolve dependencies for enabling a mod. Checks `requires` and `conflicts`
/// relationships (one level deep — no transitive resolution for v1).
pub fn resolve_enable(
    mod_entry: &ModEntry,
    game: &GameEntry,
) -> DependencyResult {
    let mut result = DependencyResult {
        can_enable: true,
        requires_enable: Vec::new(),
        must_disable: Vec::new(),
        warnings: Vec::new(),
    };

    for rel in &mod_entry.relationships {
        match rel.relation_type.as_str() {
            "requires" => {
                let target = find_target(rel, game);
                match target {
                    Some(t) if t.enabled => {
                        // Already enabled — satisfied
                    }
                    Some(t) => {
                        // Installed but not enabled
                        result.requires_enable.push(RequiredMod {
                            mod_id: Some(t.id.clone()),
                            mod_name: t.name.clone(),
                            is_installed: true,
                            is_enabled: false,
                        });
                    }
                    None => {
                        // Not installed at all
                        result.can_enable = false;
                        result.requires_enable.push(RequiredMod {
                            mod_id: None,
                            mod_name: rel.target_mod_name.clone().unwrap_or_else(|| "unknown".to_string()),
                            is_installed: false,
                            is_enabled: false,
                        });
                    }
                }
            }
            "conflicts" => {
                let target = find_target(rel, game);
                if let Some(t) = target {
                    if t.enabled {
                        result.can_enable = false;
                        result.must_disable.push(ConflictingMod {
                            mod_id: Some(t.id.clone()),
                            mod_name: t.name.clone(),
                            is_installed: true,
                            is_enabled: true,
                        });
                    }
                }
            }
            "recommends" => {
                let target = find_target(rel, game);
                if target.is_none() || target.map(|t| !t.enabled).unwrap_or(true) {
                    let name = rel.target_mod_name.clone().unwrap_or_else(|| "unknown".to_string());
                    result.warnings.push(format!("Recommends: {} (not enabled)", name));
                }
            }
            "loads_after" | "loads_before" => {
                // Soft constraint — warn if priority order is wrong
                let target = find_target(rel, game);
                if let Some(t) = target {
                    if t.enabled {
                        let my_prio = mod_entry.priority;
                        let their_prio = t.priority;
                        let ok = match rel.relation_type.as_str() {
                            "loads_after" => my_prio > their_prio,
                            "loads_before" => my_prio < their_prio,
                            _ => true,
                        };
                        if !ok {
                            result.warnings.push(format!(
                                "Load order: '{}' should be {} '{}'",
                                mod_entry.name,
                                if rel.relation_type == "loads_after" { "after" } else { "before" },
                                t.name
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    result
}

fn find_target<'a>(rel: &ModRelationEntry, game: &'a GameEntry) -> Option<&'a ModEntry> {
    if let Some(ref id) = rel.target_mod_id {
        game.mods.iter().find(|m| &m.id == id)
    } else if let Some(ref name) = rel.target_mod_name {
        game.mods.iter().find(|m| &m.name == name)
    } else {
        None
    }
}
