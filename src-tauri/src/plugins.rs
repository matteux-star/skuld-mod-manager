//! Bethesda plugin (.esp/.esm/.esl) parser and management.
//! Parses binary headers to detect plugin type, masters, and ESL flags.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub filename: String,
    pub plugin_type: String, // "esm", "esp", "esl", "esl_flagged_esp"
    pub masters: Vec<String>,
    pub description: Option<String>,
    pub record_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginValidation {
    pub plugins: Vec<PluginInfo>,
    pub warnings: Vec<String>,
    pub missing_masters: Vec<(String, Vec<String>)>, // (plugin, missing masters)
    pub total_esp_esm: usize,
    pub total_esl: usize,
}

/// Parse the header of a Bethesda plugin file (.esp/.esm/.esl).
pub fn parse_plugin_header(path: &Path) -> Result<PluginInfo, String> {
    let filename = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let mut file = fs::File::open(path).map_err(|e| format!("Open: {}", e))?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).map_err(|e| format!("Read magic: {}", e))?;

    if &magic != b"TES4" {
        return Err(format!("{}: not a valid TES4 plugin (bad magic)", filename));
    }

    // Header size (4 bytes, little-endian)
    let mut hdr_size_buf = [0u8; 4];
    file.read_exact(&mut hdr_size_buf).map_err(|e| format!("Read header size: {}", e))?;
    let _header_size = u32::from_le_bytes(hdr_size_buf) as usize;

    // Flags (4 bytes)
    let mut flags_buf = [0u8; 4];
    file.read_exact(&mut flags_buf).map_err(|e| format!("Read flags: {}", e))?;
    let flags = u32::from_le_bytes(flags_buf);

    // Determine plugin type
    let is_esm = (flags & 0x0000_0001) != 0;
    let is_esl = (flags & 0x0000_0200) != 0;
    let is_esl_flagged = (flags & 0x0000_0400) != 0;

    let plugin_type = if is_esl {
        "esl".to_string()
    } else if is_esl_flagged {
        "esl_flagged_esp".to_string()
    } else if is_esm {
        "esm".to_string()
    } else {
        "esp".to_string()
    };

    // Form count (4 bytes, at offset 0x10 relative to header start)
    let mut form_count_buf = [0u8; 4];
    // Skip to flags offset + 4 + form count field
    // Actually the simplest approach: read remaining header, scan for MAST subrecords
    // The header starts after the 24-byte fixed portion. Subrecords follow.

    // Read the rest of the header to find MAST (master) subrecords
    let mut remaining = Vec::new();
    file.read_to_end(&mut remaining).map_err(|e| format!("Read body: {}", e))?;

    let masters = parse_masters(&remaining);
    let description = parse_description(&remaining);
    let record_count = parse_record_count(&remaining);

    Ok(PluginInfo {
        filename,
        plugin_type,
        masters,
        description,
        record_count,
    })
}

/// Scan a directory for Bethesda plugin files.
pub fn scan_plugins(dir: &Path) -> Result<Vec<PluginInfo>, String> {
    let mut plugins = Vec::new();
    if !dir.is_dir() { return Ok(plugins); }

    for entry in fs::read_dir(dir).map_err(|e| format!("Read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Entry: {}", e))?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            if ext == "esp" || ext == "esm" || ext == "esl" {
                if let Ok(info) = parse_plugin_header(&path) {
                    plugins.push(info);
                }
            }
        }
    }
    Ok(plugins)
}

/// Validate a plugin set: check for missing masters and load order issues.
pub fn validate_plugins(plugins: &[PluginInfo]) -> PluginValidation {
    let mut warnings = Vec::new();
    let mut missing_masters = Vec::new();
    let mut total_esp_esm = 0usize;
    let mut total_esl = 0usize;

    // Count totals
    for p in plugins {
        match p.plugin_type.as_str() {
            "esp" | "esm" => total_esp_esm += 1,
            "esl" | "esl_flagged_esp" => total_esl += 1,
            _ => {}
        }
    }

    // Check master dependencies
    let plugin_names: Vec<&str> = plugins.iter().map(|p| p.filename.as_str()).collect();
    for p in plugins {
        let missing: Vec<String> = p.masters.iter()
            .filter(|m| !plugin_names.iter().any(|n| n.eq_ignore_ascii_case(m)))
            .cloned()
            .collect();
        if !missing.is_empty() {
            warnings.push(format!("{}: missing master(s): {:?}", p.filename, missing));
            missing_masters.push((p.filename.clone(), missing.clone()));
        }
    }

    if total_esp_esm + total_esl > 255 && total_esp_esm > 254 {
        warnings.push(format!(
            "Plugin limit warning: {} ESP+ESM (limit ~254), {} ESL (limit 4096)",
            total_esp_esm, total_esl
        ));
    }

    PluginValidation {
        plugins: plugins.to_vec(),
        warnings,
        missing_masters,
        total_esp_esm,
        total_esl,
    }
}

/// Read/write plugins.txt (Skyrim SE / Fallout 4 format).
/// Format: one plugin per line, "*" prefix means enabled.
pub fn read_plugins_txt(path: &Path) -> Result<Vec<(String, bool)>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|e| format!("Read: {}", e))?;
    let mut plugins = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(name) = line.strip_prefix('*') {
            plugins.push((name.to_string(), true));
        } else {
            plugins.push((line.to_string(), false));
        }
    }
    Ok(plugins)
}

pub fn write_plugins_txt(path: &Path, plugins: &[(String, bool)]) -> Result<(), String> {
    let mut content = String::from("# Generated by Skuld Mod Manager\n");
    // Always-enabled official files first
    for official in &["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"] {
        if plugins.iter().any(|(n, _)| n.eq_ignore_ascii_case(official)) {
            content.push_str(&format!("*{}\n", official));
        }
    }
    for (name, enabled) in plugins {
        let is_official = ["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"]
            .iter().any(|o| name.eq_ignore_ascii_case(o));
        if !is_official {
            if *enabled { content.push_str(&format!("*{}\n", name)); }
            else { content.push_str(&format!("{}\n", name)); }
        }
    }
    fs::write(path, content).map_err(|e| format!("Write: {}", e))?;
    Ok(())
}

// ═══════════════════════════════════════════════════════
// Binary parsing helpers
// ═══════════════════════════════════════════════════════

fn parse_masters(data: &[u8]) -> Vec<String> {
    let mut masters = Vec::new();
    let mut pos = 0usize;
    while pos + 10 <= data.len() {
        // Look for MAST marker
        if &data[pos..pos + 4] == b"MAST" {
            let size = u16::from_le_bytes([data[pos + 4], data[pos + 5]]) as usize;
            let name_start = pos + 6;
            let name_end = (name_start + size).min(data.len());
            if name_start < name_end {
                let name_bytes = &data[name_start..name_end];
                if let Ok(name) = std::str::from_utf8(name_bytes) {
                    let name = name.trim_end_matches('\0').to_string();
                    if !name.is_empty() {
                        masters.push(name);
                    }
                }
            }
            pos = name_end;
        } else {
            pos += 1;
        }
    }
    masters
}

fn parse_description(data: &[u8]) -> Option<String> {
    let mut pos = 0usize;
    while pos + 10 <= data.len() {
        if &data[pos..pos + 4] == b"SNAM" {
            let size = u16::from_le_bytes([data[pos + 4], data[pos + 5]]) as usize;
            let name_start = pos + 6;
            let name_end = (name_start + size).min(data.len());
            if name_start < name_end {
                let desc_bytes = &data[name_start..name_end];
                if let Ok(desc) = std::str::from_utf8(desc_bytes) {
                    let desc = desc.trim_end_matches('\0').to_string();
                    if !desc.is_empty() {
                        return Some(desc);
                    }
                }
            }
            pos = name_end;
        } else {
            pos += 1;
        }
    }
    None
}

fn parse_record_count(data: &[u8]) -> Option<u32> {
    // The GRUP count or form count varies. For simplicity, return None for now.
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_plugin_header_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("TestMod.esp");
        let mut f = fs::File::create(&path).unwrap();

        // Minimal TES4 header
        let mut data = Vec::new();
        data.extend(b"TES4");                    // magic
        data.extend(&0u32.to_le_bytes());         // header size placeholder
        data.extend(&0x0000_0400u32.to_le_bytes()); // flags: ESL-flagged (bit 0x400)
        data.extend(&1u32.to_le_bytes());         // form count (actually in subrecords, simplified)
        // MAST subrecord
        data.extend(b"MAST");
        let mast_data = b"Skyrim.esm\0";
        data.extend(&(mast_data.len() as u16).to_le_bytes()); // size including null
        data.extend(mast_data);
        // SNAM subrecord
        data.extend(b"SNAM");
        let snam_data = b"A test mod.\0";
        data.extend(&(snam_data.len() as u16).to_le_bytes());
        data.extend(snam_data);

        f.write_all(&data).unwrap();
        drop(f);

        let info = parse_plugin_header(&path).unwrap();
        assert_eq!(info.filename, "TestMod.esp");
        assert_eq!(info.plugin_type, "esl_flagged_esp");
        assert!(info.masters.contains(&"Skyrim.esm".to_string()));
        assert_eq!(info.description, Some("A test mod.".to_string()));
    }

    #[test]
    fn test_parse_plugin_header_esm() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Update.esm");
        let mut f = fs::File::create(&path).unwrap();
        let mut data = Vec::new();
        data.extend(b"TES4");
        data.extend(&0u32.to_le_bytes());
        data.extend(&0x0000_0001u32.to_le_bytes()); // ESM flag
        data.extend(&1u32.to_le_bytes());
        f.write_all(&data).unwrap();
        drop(f);

        let info = parse_plugin_header(&path).unwrap();
        assert_eq!(info.plugin_type, "esm");
    }
}
