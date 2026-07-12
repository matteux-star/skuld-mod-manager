use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{archive, config, library_dir};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
    pub id: String,
    pub game_id: String,
    pub game_type: String,
    pub mod_name: String,
    pub url: String,
    pub filename: String,
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub status: String, // "queued" | "downloading" | "extracting" | "completed" | "failed" | "cancelled"
    pub error: Option<String>,
    pub created_at: u64,
}

lazy_static::lazy_static! {
    static ref QUEUE: Mutex<Vec<DownloadJob>> = Mutex::new(Vec::new());
    static ref CANCEL_FLAGS: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
}

fn downloads_dir() -> PathBuf {
    crate::config_dir().join("downloads")
}

pub fn start_download(
    game_id: String,
    game_type: String,
    mod_name: String,
    url: String,
    filename: String,
) -> Result<DownloadJob, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let job = DownloadJob {
        id: id.clone(),
        game_id,
        game_type,
        mod_name,
        url,
        filename,
        total_bytes: None,
        downloaded_bytes: 0,
        status: "queued".to_string(),
        error: None,
        created_at: now_secs(),
    };

    // Spawn async download task
    let job_clone = job.clone();
    tauri::async_runtime::spawn(async move {
        run_download(job_clone).await;
    });

    Ok(job)
}

pub fn cancel_download(job_id: String) -> Result<(), String> {
    if let Ok(mut flags) = CANCEL_FLAGS.lock() {
        flags.insert(job_id, true);
    }
    Ok(())
}

pub fn get_download_status() -> Result<Vec<DownloadJob>, String> {
    if let Ok(queue) = QUEUE.lock() {
        Ok(queue.clone())
    } else {
        Ok(vec![])
    }
}

pub fn clear_completed_downloads() -> Result<(), String> {
    if let Ok(mut queue) = QUEUE.lock() {
        queue.retain(|j| j.status != "completed" && j.status != "failed" && j.status != "cancelled");
    }
    Ok(())
}

async fn run_download(mut job: DownloadJob) {
    // Add to queue
    {
        if let Ok(mut queue) = QUEUE.lock() {
            queue.push(job.clone());
        }
    }

    // Check cancel
    let is_cancelled = || -> bool {
        CANCEL_FLAGS.lock().ok().map(|f| f.contains_key(&job.id)).unwrap_or(false)
    };

    // Create downloads dir
    let _ = fs::create_dir_all(downloads_dir());

    let dest_path = downloads_dir().join(&job.filename);

    // Download
    job.status = "downloading".to_string();
    update_job(&job);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            job.status = "failed".to_string();
            job.error = Some(format!("HTTP client: {}", e));
            update_job(&job);
            return;
        }
    };

    let response = match client.get(&job.url).send().await {
        Ok(r) => r,
        Err(e) => {
            job.status = "failed".to_string();
            job.error = Some(format!("Download failed: {}", e));
            update_job(&job);
            return;
        }
    };

    job.total_bytes = response.content_length();
    update_job(&job);

    // Stream to file
    let mut file = match fs::File::create(&dest_path) {
        Ok(f) => f,
        Err(e) => {
            job.status = "failed".to_string();
            job.error = Some(format!("Cannot create file: {}", e));
            update_job(&job);
            return;
        }
    };

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk_result) = stream.next().await {
        if is_cancelled() {
            job.status = "cancelled".to_string();
            update_job(&job);
            let _ = fs::remove_file(&dest_path);
            return;
        }
        match chunk_result {
            Ok(bytes) => {
                if let Err(e) = file.write_all(&bytes) {
                    job.status = "failed".to_string();
                    job.error = Some(format!("Write error: {}", e));
                    update_job(&job);
                    return;
                }
                job.downloaded_bytes += bytes.len() as u64;
                update_job(&job);
            }
            Err(e) => {
                job.status = "failed".to_string();
                job.error = Some(format!("Stream error: {}", e));
                update_job(&job);
                return;
            }
        }
    }

    drop(file);

    // Extract and import
    job.status = "extracting".to_string();
    update_job(&job);

    match import_downloaded(&job, &dest_path) {
        Ok(()) => {
            job.status = "completed".to_string();
            // Clean up download file
            let _ = fs::remove_file(&dest_path);
        }
        Err(e) => {
            job.status = "failed".to_string();
            job.error = Some(e);
        }
    }
    update_job(&job);
}

fn import_downloaded(job: &DownloadJob, archive_path: &PathBuf) -> Result<(), String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let game = cfg.games.iter().find(|g| g.id == job.game_id).ok_or("Game not found")?;
    let game_type = game.game_type.clone();

    // Deduplicate name
    let mod_name = if game.mods.iter().any(|m| m.name == job.mod_name) {
        format!("{} (download)", job.mod_name)
    } else {
        job.mod_name.clone()
    };

    let (extract_dir, _) = archive::extract_archive(archive_path, &game_type, &mod_name)?;

    let installed_files = match game_type.as_str() {
        "sod2" => {
            let _ = archive::validate_sod2(&extract_dir)?;
            archive::derive_installed_files(&extract_dir, "sod2", &mod_name)?
        }
        "witcher3" => {
            let mod_root = archive::validate_witcher3(&extract_dir)?;
            archive::derive_installed_files(&mod_root, "witcher3", &mod_name)?
        }
        other => archive::derive_installed_files(&extract_dir, other, &mod_name)?,
    };

    if installed_files.is_empty() {
        return Err("No valid mod files found in download".to_string());
    }

    let game = cfg.games.iter_mut().find(|g| g.id == job.game_id).unwrap();
    let priority = game.mods.len() + 1;
    let archive_name = archive_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "download".to_string());

    game.mods.push(config::ModEntry {
        id: uuid::Uuid::new_v4().to_string(),
        name: mod_name,
        archive_source: archive_name,
        enabled: false,
        priority,
        installed_files,
        version: None,
        author: None,
        description: None,
        source_url: Some(job.url.clone()),
        category: None,
        tags: vec![],
        installed_at: Some(format_ts_simple(now_secs())),
        updated_at: None,
        relationships: vec![],
    });

    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(())
}

fn update_job(job: &DownloadJob) {
    if let Ok(mut queue) = QUEUE.lock() {
        if let Some(existing) = queue.iter_mut().find(|j| j.id == job.id) {
            *existing = job.clone();
        } else {
            queue.push(job.clone());
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn format_ts_simple(seconds: u64) -> String {
    crate::format_ts(seconds)
}
