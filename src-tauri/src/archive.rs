use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::library_dir;

/// Sanitise a path extracted from an archive. Returns None if the path:
/// - Contains `..` components
/// - Resolves outside the target directory
/// - Is absolute
fn sanitise_entry_path(entry_path: &str, target_dir: &Path) -> Option<PathBuf> {
    if entry_path.contains("..") {
        return None;
    }
    let path = PathBuf::from(entry_path);
    if path.is_absolute() {
        return None;
    }
    let resolved = target_dir.join(&path);
    // Canonicalise to check it's still under target_dir
    match resolved.canonicalize() {
        Ok(canon) => {
            if canon.starts_with(target_dir) {
                Some(path)
            } else {
                None
            }
        }
        Err(_) => {
            // Path doesn't exist yet — check logically
            let mut current = target_dir.to_path_buf();
            for component in path.components() {
                current.push(component);
            }
            // The resolved path should start with target_dir
            // A simple check: the target_dir must be a prefix
            if current.starts_with(target_dir) {
                Some(path)
            } else {
                None
            }
        }
    }
}

/// Extract a zip archive to `target_dir`. Returns list of extracted relative paths.
pub fn extract_zip(archive_path: &Path, target_dir: &Path) -> Result<Vec<PathBuf>, String> {
    fs::create_dir_all(target_dir)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;

    let file = fs::File::open(archive_path)
        .map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip: {}", e))?;

    let mut extracted = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("Failed to read zip entry {}: {}", i, e))?;

        let entry_name = entry.name().to_string();
        let relative = match sanitise_entry_path(&entry_name, target_dir) {
            Some(p) => p,
            None => {
                return Err(format!(
                    "Rejected unsafe archive path: '{}'. Archive may contain path-traversal.",
                    entry_name
                ));
            }
        };

        if entry.is_dir() {
            fs::create_dir_all(target_dir.join(&relative))
                .map_err(|e| format!("Failed to create dir: {}", e))?;
        } else {
            if let Some(parent) = target_dir.join(&relative).parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dir: {}", e))?;
            }
            let mut outfile = fs::File::create(target_dir.join(&relative))
                .map_err(|e| format!("Failed to create file: {}", e))?;
            io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;
            extracted.push(relative);
        }
    }

    Ok(extracted)
}

/// Find the `7z` binary on the system. Returns None if not found.
fn find_7z() -> Option<String> {
    for name in &["7z", "7za", "7zr"] {
        if Command::new(name).arg("--help").output().is_ok() {
            return Some(name.to_string());
        }
    }
    // Check common paths
    for path in &["/usr/bin/7z", "/usr/bin/7za", "/usr/local/bin/7z"] {
        if Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    None
}

/// Check if 7z/p7zip is available. Returns the binary name or an error.
pub fn check_7z_available() -> Result<String, String> {
    find_7z().ok_or_else(|| {
        "7z/p7zip not found. Install p7zip or p7zip-full to import .7z and .rar archives.".to_string()
    })
}

/// Extract a 7z or rar archive to `target_dir` using system `7z`.
pub fn extract_7z(archive_path: &Path, target_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let binary = check_7z_available()?;
    fs::create_dir_all(target_dir)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;

    let output = Command::new(&binary)
        .arg("x")
        .arg(archive_path)
        .arg(&format!("-o{}", target_dir.display()))
        .arg("-y") // auto-yes
        .output()
        .map_err(|e| format!("Failed to run {}: {}", binary, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Archive extraction failed: {}", stderr));
    }

    // Walk the target dir to get extracted paths
    let mut files = Vec::new();
    walk_extracted(target_dir, target_dir, &mut files)
        .map_err(|e| format!("Failed to read extracted files: {}", e))?;

    Ok(files)
}

fn walk_extracted(base: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_extracted(base, &path, files)?;
        } else {
            if let Ok(relative) = path.strip_prefix(base) {
                files.push(relative.to_path_buf());
            }
        }
    }
    Ok(())
}

/// Determine the archive type from file extension.
pub enum ArchiveType {
    Zip,
    SevenZip,
    Rar,
}

impl ArchiveType {
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("zip") => Some(Self::Zip),
            Some("7z") => Some(Self::SevenZip),
            Some("rar") => Some(Self::Rar),
            _ => None,
        }
    }
}

/// Extract an archive of any supported type to the library.
/// Returns the path where files were extracted and the list of relative paths.
pub fn extract_archive(
    archive_path: &Path,
    game_type: &str,
    mod_name: &str,
) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let lib_dir = library_dir().join(game_type).join(mod_name);
    let archive_type = ArchiveType::from_path(archive_path)
        .ok_or_else(|| "Unsupported archive format. Use .zip, .7z, or .rar.".to_string())?;

    let extracted = match archive_type {
        ArchiveType::Zip => extract_zip(archive_path, &lib_dir)?,
        ArchiveType::SevenZip | ArchiveType::Rar => extract_7z(archive_path, &lib_dir)?,
    };

    Ok((lib_dir, extracted))
}

/// Validate mod format for State of Decay 2: find .pak files at any depth.
/// Returns the relative paths of all .pak files found.
pub fn validate_sod2(extract_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pak_files = Vec::new();
    find_files_by_ext(extract_dir, extract_dir, "pak", &mut pak_files)
        .map_err(|e| format!("Failed to scan extracted files: {}", e))?;

    if pak_files.is_empty() {
        return Err("No .pak files found in archive. State of Decay 2 mods must contain .pak files.".to_string());
    }

    Ok(pak_files)
}

/// Validate mod format for The Witcher 3: find a folder containing `content/`.
/// Returns the path to the mod root (the folder that has content/).
pub fn validate_witcher3(extract_dir: &Path) -> Result<PathBuf, String> {
    // First, check if extract_dir itself has content/
    if extract_dir.join("content").is_dir() {
        return Ok(extract_dir.to_path_buf());
    }

    // Search one level deep
    for entry in fs::read_dir(extract_dir)
        .map_err(|e| format!("Failed to read extract dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() && path.join("content").is_dir() {
            return Ok(path);
        }
    }

    Err("No mod folder with a 'content/' subfolder found. Witcher 3 mods must contain a folder with 'content/' inside.".to_string())
}

/// Derive installed_files: walk the mod root and record paths relative to the game's mod root.
/// For SoD2, each .pak file maps to `<game_mod_path>/<pak_filename>`.
/// For Witcher 3, each file maps to `<game_mod_path>/<mod_name>/<relative_path>`.
pub fn derive_installed_files(
    mod_root: &Path,
    game_type: &str,
    mod_name: &str,
) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    walk_relative(mod_root, mod_root, &mut files)
        .map_err(|e| format!("Failed to walk mod root: {}", e))?;

    let installed: Vec<String> = match game_type {
        "sod2" => files
            .into_iter()
            .filter(|p| p.extension().map(|e| e == "pak").unwrap_or(false))
            .map(|p| format!("Content/Paks/~mods/{}", p.file_name().unwrap().to_string_lossy()))
            .collect(),
        "witcher3" => files
            .into_iter()
            .map(|p| format!("Mods/{}/{}", mod_name, p.display()))
            .collect(),
        _ => files.into_iter().map(|p| p.display().to_string()).collect(),
    };

    Ok(installed)
}

fn find_files_by_ext(base: &Path, dir: &Path, ext: &str, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            find_files_by_ext(base, &path, ext, files)?;
        } else if path.extension().map(|e| e == ext).unwrap_or(false) {
            if let Ok(relative) = path.strip_prefix(base) {
                files.push(relative.to_path_buf());
            }
        }
    }
    Ok(())
}

fn walk_relative(base: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_relative(base, &path, files)?;
        } else {
            if let Ok(relative) = path.strip_prefix(base) {
                files.push(relative.to_path_buf());
            }
        }
    }
    Ok(())
}
