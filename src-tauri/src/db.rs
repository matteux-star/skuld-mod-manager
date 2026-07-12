//! SQLite-backed persistence. Replaces config.rs JSON file with a relational DB.
//! Frontend-facing API (command signatures) remains identical.
//! Uses rusqlite with bundled SQLite — no system dependency.

use rusqlite::{Connection, params};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::config::{AppConfig, GameEntry, ModEntry, Profile, ModState, ModRelationEntry};
use crate::config_dir;

lazy_static::lazy_static! {
    static ref DB: Mutex<Option<Connection>> = Mutex::new(None);
}

fn db_path() -> PathBuf {
    config_dir().join("skuld.db")
}

pub fn init() -> Result<(), String> {
    let path = db_path();
    let is_new = !path.exists();

    let conn = Connection::open(&path).map_err(|e| format!("DB open: {}", e))?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| format!("PRAGMA: {}", e))?;

    if is_new {
        create_schema(&conn)?;
        // Try migration from JSON
        let _ = migrate_json(&conn);
    } else {
        create_schema(&conn)?; // ensure schema exists (idempotent)
    }

    *DB.lock().map_err(|e| format!("DB lock: {}", e))? = Some(conn);
    Ok(())
}

fn conn() -> Result<std::sync::MutexGuard<'static, Option<Connection>>, String> {
    DB.lock().map_err(|e| format!("DB lock: {}", e))
}

fn create_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS games (
            id TEXT PRIMARY KEY,
            game_type TEXT NOT NULL,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            launch_path TEXT,
            support_status TEXT NOT NULL DEFAULT 'verified',
            active_profile_id TEXT
        );
        CREATE TABLE IF NOT EXISTS mods (
            id TEXT PRIMARY KEY,
            game_id TEXT NOT NULL,
            name TEXT NOT NULL,
            archive_source TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 0,
            priority INTEGER NOT NULL DEFAULT 0,
            version TEXT,
            author TEXT,
            description TEXT,
            source_url TEXT,
            category TEXT,
            installed_at TEXT,
            updated_at TEXT,
            FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS installed_files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mod_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS profiles (
            id TEXT PRIMARY KEY,
            game_id TEXT NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS profile_mod_states (
            profile_id TEXT NOT NULL,
            mod_id TEXT NOT NULL,
            enabled INTEGER NOT NULL,
            priority INTEGER NOT NULL,
            PRIMARY KEY (profile_id, mod_id),
            FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
            FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS mod_tags (
            mod_id TEXT NOT NULL,
            tag TEXT NOT NULL,
            PRIMARY KEY (mod_id, tag),
            FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS mod_relationships (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_mod_id TEXT NOT NULL,
            target_mod_id TEXT,
            target_mod_name TEXT,
            relation_type TEXT NOT NULL,
            note TEXT,
            FOREIGN KEY (source_mod_id) REFERENCES mods(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );"
    ).map_err(|e| format!("Schema: {}", e))?;

    // Set schema version if not present
    conn.execute(
        "INSERT OR IGNORE INTO schema_version (version) VALUES (1)",
        [],
    ).map_err(|e| format!("Version: {}", e))?;

    Ok(())
}

fn migrate_json(conn: &Connection) -> Result<(), String> {
    let json_path = config_dir().join("config.json");
    if !json_path.exists() { return Ok(()); }

    let data = fs::read_to_string(&json_path).map_err(|e| format!("Read JSON: {}", e))?;
    let cfg: AppConfig = serde_json::from_str(&data).map_err(|e| format!("Parse JSON: {}", e))?;

    let tx = conn.unchecked_transaction().map_err(|e| format!("Tx: {}", e))?;

    for game in &cfg.games {
        tx.execute(
            "INSERT OR REPLACE INTO games (id, game_type, name, path, launch_path, support_status, active_profile_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![game.id, game.game_type, game.name, game.path, game.launch_path, game.support_status, game.active_profile_id],
        ).map_err(|e| format!("Game insert: {}", e))?;

        for m in &game.mods {
            tx.execute(
                "INSERT OR REPLACE INTO mods (id, game_id, name, archive_source, enabled, priority, version, author, description, source_url, category, installed_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![m.id, game.id, m.name, m.archive_source, m.enabled as i32, m.priority as i64, m.version, m.author, m.description, m.source_url, m.category, m.installed_at, m.updated_at],
            ).map_err(|e| format!("Mod insert: {}", e))?;

            for f in &m.installed_files {
                tx.execute(
                    "INSERT INTO installed_files (mod_id, file_path) VALUES (?1, ?2)",
                    params![m.id, f],
                ).map_err(|e| format!("File insert: {}", e))?;
            }
            for t in &m.tags {
                tx.execute(
                    "INSERT OR IGNORE INTO mod_tags (mod_id, tag) VALUES (?1, ?2)",
                    params![m.id, t],
                ).map_err(|e| format!("Tag insert: {}", e))?;
            }
            for r in &m.relationships {
                tx.execute(
                    "INSERT INTO mod_relationships (source_mod_id, target_mod_id, target_mod_name, relation_type, note)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![m.id, r.target_mod_id, r.target_mod_name, r.relation_type, r.note],
                ).map_err(|e| format!("Rel insert: {}", e))?;
            }
        }

        for p in &game.profiles {
            tx.execute(
                "INSERT OR REPLACE INTO profiles (id, game_id, name, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![p.id, game.id, p.name, p.created_at],
            ).map_err(|e| format!("Profile insert: {}", e))?;

            for ms in &p.mod_states {
                tx.execute(
                    "INSERT OR REPLACE INTO profile_mod_states (profile_id, mod_id, enabled, priority) VALUES (?1, ?2, ?3, ?4)",
                    params![p.id, ms.mod_id, ms.enabled as i32, ms.priority as i64],
                ).map_err(|e| format!("PMS insert: {}", e))?;
            }
        }
    }

    tx.commit().map_err(|e| format!("Commit: {}", e))?;

    // Rename JSON as backup
    let _ = fs::rename(&json_path, json_path.with_extension("json.v3.bak"));
    Ok(())
}

// ═══════════════════════════════════════════════════════
// Public API — mirrors config.rs
// ═══════════════════════════════════════════════════════

pub fn load_config() -> Result<AppConfig, String> {
    let guard = conn()?;
    let conn = guard.as_ref().ok_or("DB not initialized")?;

    let mut games = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT id, game_type, name, path, launch_path, support_status, active_profile_id FROM games"
    ).map_err(|e| format!("Query: {}", e))?;

    let rows = stmt.query_map([], |row| {
        Ok(GameEntry {
            id: row.get(0)?,
            game_type: row.get(1)?,
            name: row.get(2)?,
            path: row.get(3)?,
            launch_path: row.get(4)?,
            support_status: row.get(5)?,
            active_profile_id: row.get(6)?,
            mods: Vec::new(),
            profiles: Vec::new(),
        })
    }).map_err(|e| format!("Query: {}", e))?;

    for game_result in rows {
        let mut game = game_result.map_err(|e| format!("Row: {}", e))?;

        // Load mods
        let mut mstmt = conn.prepare(
            "SELECT id, name, archive_source, enabled, priority, version, author, description, source_url, category, installed_at, updated_at FROM mods WHERE game_id = ?1"
        ).map_err(|e| format!("Mod query: {}", e))?;

        let mod_rows = mstmt.query_map(params![game.id], |row| {
            Ok(ModEntry {
                id: row.get(0)?,
                name: row.get(1)?,
                archive_source: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                priority: row.get::<_, i64>(4)? as usize,
                installed_files: Vec::new(),
                version: row.get(5)?,
                author: row.get(6)?,
                description: row.get(7)?,
                source_url: row.get(8)?,
                category: row.get(9)?,
                tags: Vec::new(),
                installed_at: row.get(10)?,
                updated_at: row.get(11)?,
                relationships: Vec::new(),
            })
        }).map_err(|e| format!("Mod query: {}", e))?;

        for mod_result in mod_rows {
            let mut m = mod_result.map_err(|e| format!("Row: {}", e))?;

            // Files
            let mut fstmt = conn.prepare("SELECT file_path FROM installed_files WHERE mod_id = ?1")
                .map_err(|e| format!("File query: {}", e))?;
            let files: Vec<String> = fstmt.query_map(params![m.id], |row| row.get(0))
                .map_err(|e| format!("File query: {}", e))?
                .filter_map(|r| r.ok())
                .collect();
            m.installed_files = files;

            // Tags
            let mut tstmt = conn.prepare("SELECT tag FROM mod_tags WHERE mod_id = ?1")
                .map_err(|e| format!("Tag query: {}", e))?;
            let tags: Vec<String> = tstmt.query_map(params![m.id], |row| row.get(0))
                .map_err(|e| format!("Tag query: {}", e))?
                .filter_map(|r| r.ok())
                .collect();
            m.tags = tags;

            // Relationships
            let mut rstmt = conn.prepare("SELECT target_mod_id, target_mod_name, relation_type, note FROM mod_relationships WHERE source_mod_id = ?1")
                .map_err(|e| format!("Rel query: {}", e))?;
            let rels: Vec<ModRelationEntry> = rstmt.query_map(params![m.id], |row| {
                Ok(ModRelationEntry {
                    target_mod_id: row.get(0)?,
                    target_mod_name: row.get(1)?,
                    relation_type: row.get(2)?,
                    note: row.get(3)?,
                })
            }).map_err(|e| format!("Rel query: {}", e))?
                .filter_map(|r| r.ok())
                .collect();
            m.relationships = rels;

            game.mods.push(m);
        }

        // Load profiles
        let mut pstmt = conn.prepare("SELECT id, name, game_id, created_at FROM profiles WHERE game_id = ?1")
            .map_err(|e| format!("Prof query: {}", e))?;
        let prof_rows = pstmt.query_map(params![game.id], |row| {
            Ok(Profile {
                id: row.get(0)?,
                name: row.get(1)?,
                game_id: row.get(2)?,
                mod_states: Vec::new(),
                created_at: row.get(3)?,
            })
        }).map_err(|e| format!("Prof query: {}", e))?;

        for prof_result in prof_rows {
            let mut p = prof_result.map_err(|e| format!("Row: {}", e))?;
            // Profile mod states
            let mut ms_stmt = conn.prepare("SELECT mod_id, enabled, priority FROM profile_mod_states WHERE profile_id = ?1")
                .map_err(|e| format!("PMS query: {}", e))?;
            let ms_rows: Vec<ModState> = ms_stmt.query_map(params![p.id], |row| {
                Ok(ModState {
                    mod_id: row.get(0)?,
                    enabled: row.get::<_, i32>(1)? != 0,
                    priority: row.get::<_, i64>(2)? as usize,
                })
            }).map_err(|e| format!("PMS query: {}", e))?
                .filter_map(|r| r.ok())
                .collect();
            p.mod_states = ms_rows;
            game.profiles.push(p);
        }

        games.push(game);
    }

    Ok(AppConfig { version: 5, games })
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let guard = conn()?;
    let conn_ = guard.as_ref().ok_or("DB not initialized")?;
    let tx = conn_.unchecked_transaction().map_err(|e| format!("Tx: {}", e))?;

    // Clear existing data (cascade handles mods/files/tags/rels)
    tx.execute("DELETE FROM games", []).map_err(|e| format!("Clear: {}", e))?;

    for game in &cfg.games {
        tx.execute(
            "INSERT INTO games (id, game_type, name, path, launch_path, support_status, active_profile_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![game.id, game.game_type, game.name, game.path, game.launch_path, game.support_status, game.active_profile_id],
        ).map_err(|e| format!("Game save: {}", e))?;

        for m in &game.mods {
            tx.execute(
                "INSERT INTO mods (id, game_id, name, archive_source, enabled, priority, version, author, description, source_url, category, installed_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![m.id, game.id, m.name, m.archive_source, m.enabled as i32, m.priority as i64, m.version, m.author, m.description, m.source_url, m.category, m.installed_at, m.updated_at],
            ).map_err(|e| format!("Mod save: {}", e))?;
            for f in &m.installed_files {
                tx.execute("INSERT INTO installed_files (mod_id, file_path) VALUES (?1, ?2)", params![m.id, f])
                    .map_err(|e| format!("File save: {}", e))?;
            }
            for t in &m.tags {
                tx.execute("INSERT OR IGNORE INTO mod_tags (mod_id, tag) VALUES (?1, ?2)", params![m.id, t])
                    .map_err(|e| format!("Tag save: {}", e))?;
            }
            for r in &m.relationships {
                tx.execute(
                    "INSERT INTO mod_relationships (source_mod_id, target_mod_id, target_mod_name, relation_type, note)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![m.id, r.target_mod_id, r.target_mod_name, r.relation_type, r.note],
                ).map_err(|e| format!("Rel save: {}", e))?;
            }
        }

        for p in &game.profiles {
            tx.execute("INSERT INTO profiles (id, game_id, name, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![p.id, game.id, p.name, p.created_at])
                .map_err(|e| format!("Prof save: {}", e))?;
            for ms in &p.mod_states {
                tx.execute("INSERT INTO profile_mod_states (profile_id, mod_id, enabled, priority) VALUES (?1, ?2, ?3, ?4)",
                    params![p.id, ms.mod_id, ms.enabled as i32, ms.priority as i64])
                    .map_err(|e| format!("PMS save: {}", e))?;
            }
        }
    }

    tx.commit().map_err(|e| format!("Commit: {}", e))?;
    Ok(())
}

/// In-place update: toggle a single mod's enabled state. Much faster than full save.
pub fn toggle_mod_enabled(game_id: &str, mod_id: &str, enabled: bool) -> Result<(), String> {
    let guard = conn()?;
    let conn_ = guard.as_ref().ok_or("DB not initialized")?;
    conn_.execute(
        "UPDATE mods SET enabled = ?1 WHERE id = ?2 AND game_id = ?3",
        params![enabled as i32, mod_id, game_id],
    ).map_err(|e| format!("Toggle: {}", e))?;
    Ok(())
}
