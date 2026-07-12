use crate::config::{AppConfig, ModEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    pub mod_id: String,
    pub mod_name: String,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub update_url: Option<String>,
    pub is_update_available: bool,
    pub error: Option<String>,
    pub source: String, // "github" | "nexus" | "manual" | "none"
}

/// Check GitHub releases for a single mod.
/// URL format: https://api.github.com/repos/{owner}/{repo}/releases/latest
async fn check_github(
    mod_entry: &ModEntry,
    repo: &str,
    client: &reqwest::Client,
) -> UpdateResult {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    match client
        .get(&url)
        .header("User-Agent", "skuld-mod-manager/0.1")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let tag = json["tag_name"].as_str().unwrap_or("").trim_start_matches('v');
                let html_url = json["html_url"].as_str().unwrap_or(&url).to_string();
                let is_update = mod_entry.version.as_deref() != Some(tag);
                return UpdateResult {
                    mod_id: mod_entry.id.clone(),
                    mod_name: mod_entry.name.clone(),
                    current_version: mod_entry.version.clone(),
                    latest_version: if tag.is_empty() { None } else { Some(tag.to_string()) },
                    update_url: Some(html_url),
                    is_update_available: is_update && !tag.is_empty(),
                    error: None,
                    source: "github".to_string(),
                };
            }
            UpdateResult {
                mod_id: mod_entry.id.clone(),
                mod_name: mod_entry.name.clone(),
                current_version: mod_entry.version.clone(),
                latest_version: None,
                update_url: None,
                is_update_available: false,
                error: Some("Failed to parse GitHub response".to_string()),
                source: "github".to_string(),
            }
        }
        Err(e) => UpdateResult {
            mod_id: mod_entry.id.clone(),
            mod_name: mod_entry.name.clone(),
            current_version: mod_entry.version.clone(),
            latest_version: None,
            update_url: None,
            is_update_available: false,
            error: Some(format!("GitHub API error: {}", e)),
            source: "github".to_string(),
        },
    }
}

/// Check a manual URL for a version string using regex.
async fn check_manual(
    mod_entry: &ModEntry,
    url: &str,
    version_regex: &str,
    client: &reqwest::Client,
) -> UpdateResult {
    match client
        .get(url)
        .header("User-Agent", "skuld-mod-manager/0.1")
        .send()
        .await
    {
        Ok(resp) => {
            match resp.text().await {
                Ok(body) => {
                    // Simple regex extraction
                    let latest = extract_version(&body, version_regex);
                    let is_update = match (&mod_entry.version, &latest) {
                        (Some(current), Some(latest)) => current != latest,
                        _ => latest.is_some(),
                    };
                    UpdateResult {
                        mod_id: mod_entry.id.clone(),
                        mod_name: mod_entry.name.clone(),
                        current_version: mod_entry.version.clone(),
                        latest_version: latest,
                        update_url: Some(url.to_string()),
                        is_update_available: is_update,
                        error: None,
                        source: "manual".to_string(),
                    }
                }
                Err(e) => UpdateResult {
                    mod_id: mod_entry.id.clone(),
                    mod_name: mod_entry.name.clone(),
                    current_version: mod_entry.version.clone(),
                    latest_version: None,
                    update_url: None,
                    is_update_available: false,
                    error: Some(format!("Failed to read response: {}", e)),
                    source: "manual".to_string(),
                },
            }
        }
        Err(e) => UpdateResult {
            mod_id: mod_entry.id.clone(),
            mod_name: mod_entry.name.clone(),
            current_version: mod_entry.version.clone(),
            latest_version: None,
            update_url: None,
            is_update_available: false,
            error: Some(format!("HTTP error: {}", e)),
            source: "manual".to_string(),
        },
    }
}

fn extract_version(body: &str, pattern: &str) -> Option<String> {
    // Simple: find pattern in body, return first capture group
    // Support both literal strings and basic regex patterns like "v(\d+\.\d+\.\d+)"
    let body = if body.len() > 50000 { &body[..50000] } else { body };

    // Try as literal substring first
    if let Some(pos) = body.find(pattern) {
        return Some(body[pos..].split_whitespace().next()?.to_string());
    }

    // Try simple regex: look for version-like patterns near the pattern string
    let prefix_patterns = ["v", "version", "tag"];
    for prefix in prefix_patterns {
        if let Some(idx) = body.to_lowercase().find(prefix) {
            let after = &body[idx + prefix.len()..];
            let version = after.trim_start_matches(&[' ', ':', '=', '"', '\''][..])
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect::<String>();
            if version.contains('.') && version.len() >= 3 {
                return Some(version);
            }
        }
    }

    None
}

/// Check all mods in a game for updates.
/// Returns results for mods that have source_url or github-compatible URLs.
pub async fn check_updates(
    game_id: &str,
    cfg: &AppConfig,
) -> Result<Vec<UpdateResult>, String> {
    let game = cfg.games.iter().find(|g| g.id == game_id).ok_or("Game not found")?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let mut results = Vec::new();
    for mod_entry in &game.mods {
        // Skip mods without enough data to check
        if let Some(ref url) = mod_entry.source_url {
            // GitHub detection: https://github.com/owner/repo
            if url.contains("github.com") {
                if let Some(repo) = url.strip_prefix("https://github.com/") {
                    let repo = repo.trim_end_matches('/');
                    results.push(check_github(mod_entry, repo, &client).await);
                    continue;
                }
            }

            // Nexus detection: https://www.nexusmods.com/game/mods/1234
            if url.contains("nexusmods.com") {
                // Nexus API requires auth key — skip for now, show "needs API key"
                results.push(UpdateResult {
                    mod_id: mod_entry.id.clone(),
                    mod_name: mod_entry.name.clone(),
                    current_version: mod_entry.version.clone(),
                    latest_version: None,
                    update_url: None,
                    is_update_available: false,
                    error: Some("Nexus Mods requires an API key. Set it in Settings.".to_string()),
                    source: "nexus".to_string(),
                });
                continue;
            }

            // Manual check: if we have a version, try fetching the URL
            if mod_entry.version.is_some() {
                results.push(check_manual(mod_entry, url, "version", &client).await);
            } else {
                results.push(UpdateResult {
                    mod_id: mod_entry.id.clone(),
                    mod_name: mod_entry.name.clone(),
                    current_version: None,
                    latest_version: None,
                    update_url: None,
                    is_update_available: false,
                    error: Some("No current version set — cannot check for updates".to_string()),
                    source: "manual".to_string(),
                });
            }
        }
    }

    Ok(results)
}
