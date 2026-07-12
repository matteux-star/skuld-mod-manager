# Skuld Mod Manager — Feature Roadmap & Design Plan

> Each feature: summary, design, integration points, edge cases, hardening analysis.
> Ordered by priority tier. Features within a tier are ordered by dependency chain.

---

## Tier 1 — Usability at Scale

---

### 1. Mod Search & Filter

#### Summary
Add text search and sort controls to the mod list so users with 30+ mods can locate and organize mods efficiently.

#### Why
Current `ModList` renders every mod as a flat scrollable list. No search, no sort. At 50+ mods (common for Witcher 3), finding a specific mod means visually scanning the entire list. This is the single biggest usability gap.

#### Current State
- `ModList.tsx` maps `game.mods` directly into `ModCard` components
- No filtering, no sorting UI exists
- Mod list order: default priority order for load-order games, insertion order for others

#### Design

**Frontend:**

New state in `ModList.tsx`:
```
searchQuery: string
sortBy: 'name' | 'status' | 'priority' | 'source'
sortDirection: 'asc' | 'desc'
```

New toolbar section above the mod list:
- Search input (text, with clear button) — filters mods in real-time as user types
- Sort dropdown next to search — name/status/priority/source, asc/desc toggle

Filtering logic:
- Client-side only — filter `game.mods` array before mapping to `ModCard`
- Search matches against: mod name, archive source filename, installed file paths
- Case-insensitive substring match
- If search yields zero results, show "No mods match '{query}'" empty state with a clear-filter button

Sort behavior:
- `name`: alphabetical by mod name
- `status`: enabled first, then disabled, then blocked; within groups by name
- `priority`: numeric priority (load-order games only; disabled mods at bottom)
- `source`: alphabetical by archive filename
- For non-load-order games, `priority` sort option is hidden

**Component changes:**
- New `SearchBar` sub-component (inline in `ModList` or separate tiny component)
- Modify the mod-list empty state to distinguish "no mods imported" from "no mods match filter"

**Backend:**
- No changes needed. Filtering/sorting is pure UI state on an already-loaded array.

#### Integration Points
- `ModList.tsx`: add search input + sort dropdown to toolbar area (between mod count badge and action buttons)
- `ModCard.tsx`: no changes (receives filtered list)
- No backend changes
- No config changes

#### Edge Cases & Risks
- **Load-order games**: search/sort changes visual order but must not affect actual priority. Dragging still writes to `reorder_mods`. Sort is display-only.
- **Rapid typing**: debounce search input (150ms) to avoid jank with large mod lists
- **Unicode names**: mod names from archives may contain non-ASCII characters. Use locale-aware comparison (`String.localeCompare`)
- **Empty states**: three different empty states now: no mods imported, no mods match filter, all mods filtered out after deploy/purge

#### Hardening Analysis
- **Performance**: 500 mods × substring search = negligible. No need for memoization beyond React's default. If profiling shows jank, wrap filter in `useMemo` keyed on `[game.mods, searchQuery, sortBy, sortDirection]`.
- **Accessibility**: search input needs `aria-label`, sort dropdown needs keyboard navigation. Focus should return to search after sort selection.
- **Persistence**: sort preference should NOT persist across game switches — reset to default when `selectedGameId` changes. If users complain, add per-game sort preference to config later.
- **State reset**: clearing search when switching games prevents stale filter state.

#### Implementation Phases
1. Add search input + debounce logic to `ModList.tsx`
2. Add sort dropdown
3. Add "no results" empty state variant
4. Test with 100+ mod fixture data

---

### 2. Mod Profiles

#### Summary
Let users save, switch, and delete named profiles — each profile stores the enabled/disabled state and priority order of all mods for a game.

#### Why
Users run different mod setups for different playthroughs. Without profiles, switching means manually toggling 40+ mods and reordering priorities. This is the #1 feature request for any mod manager that has survived past "basic toggle" stage.

#### Current State
- `GameEntry` has a flat `mods: Vec<ModEntry>` — one set of states per game
- Enabling/disabling modifies `ModEntry.enabled` and `ModEntry.priority` in place
- No concept of named configuration sets

#### Design

**Data model (config v3):**

Add to `AppConfig`:
```rust
struct Profile {
    id: String,           // UUID
    name: String,
    game_id: String,      // which game this profile belongs to
    mod_states: Vec<ModState>,  // snapshot of enabled/priority per mod
    created_at: String,   // ISO 8601
}
```

`ModState`:
```rust
struct ModState {
    mod_id: String,       // matches ModEntry.id
    enabled: bool,
    priority: u32,
}
```

`GameEntry` gains:
```rust
active_profile_id: Option<String>,  // None = no profile active (ad-hoc state)
profiles: Vec<Profile>,
```

**How profiles work:**

- **Ad-hoc mode** (no profile selected): works exactly like today. Toggle/reorder writes directly to `GameEntry.mods`. This is the initial state for existing users after migration.
- **Profile active**: when user creates or switches to a profile, `GameEntry.mods.enabled` and `GameEntry.mods.priority` are overwritten from the profile's `mod_states`. Subsequent toggles write to both `GameEntry.mods` AND `active_profile.mod_states`.
- **Deploy uses current state**: `deploy_all`/`purge_all` always deploy whatever `GameEntry.mods` currently says — whether ad-hoc or profile-driven. No change to deploy logic.
- **Profile is a snapshot**: saving a profile captures current enabled/priority for ALL mods in the game (not just enabled ones). This means restoring a profile can disable mods too.
- **Mod add/delete while profile active**: adding a new mod — it enters `mod_states` as disabled, priority 0. Deleting a mod — remove its entry from `mod_states`. Profile stays in sync.
- **Profile switching**: purges current symlinks, applies new profile's states, re-deploys. User sees a toast: "Switched to profile '{name}' — X enabled, Y disabled."

**Stale profile detection:**
If a profile's `mod_states` references a mod ID that no longer exists (mod was deleted), silently skip that entry on restore. Show a small warning badge on the profile: "1 missing mod."

**Frontend:**

New UI in `ModList` panel header or toolbar:
- Profile dropdown (left of Launch button): shows active profile name or "No profile" when ad-hoc
- Dropdown items: switch profile, save current as new, overwrite active, rename, delete, manage profiles

New `ProfileDialog` component:
- List all profiles for the game
- Create new: name input + "Save current mod states as profile"
- Rename: inline edit
- Delete: confirmation dialog (mods stay, just the profile snapshot is deleted)
- Shows mod count, last-modified timestamp per profile

**Backend commands:**

| Command | Purpose |
|---|---|
| `create_profile(game_id, name)` | Snapshot current mod states into new profile. Set as active. |
| `apply_profile(game_id, profile_id)` | Purge current, load profile states, redeploy, set active. |
| `delete_profile(game_id, profile_id)` | Remove profile. If was active, switch to ad-hoc (keep current states). |
| `update_profile(game_id, profile_id, name)` | Rename profile. |
| `list_profiles(game_id)` | Return all profiles for a game (already in config, just filter). |

**Actually — simpler approach:** Profiles live in `AppConfig` alongside games. All profile operations are config mutations: read-modify-write. Only `apply_profile` needs deploy logic. This avoids 5 new commands; `apply_profile` is the only new Tauri command. The rest are config manipulations the frontend can do by calling `get_config` + a generic `save_config` (or we add a few targeted commands for atomicity).

**Decision: add 2 new commands, not 5:**
- `apply_profile(game_id, profile_id)` — the heavy one (purge + load + deploy)
- `save_config(config)` — generic config save (enables all CRUD on profiles from frontend without a command per operation)

This keeps the backend thin and lets the frontend own profile CRUD.

#### Integration Points
- `config.rs`: add `Profile` struct, add `profiles` and `active_profile_id` to `GameEntry`. Config version bump to v3 with migration (v2 games get empty profiles vec, `None` active).
- `lib.rs`: add `apply_profile` and `save_config` commands
- `deploy.rs`: no changes (deploy reads current `GameEntry.mods`, which `apply_profile` mutates before calling deploy)
- `App.tsx`: new profile state management, new dialog handler
- `ModList.tsx`: profile dropdown in header
- New file: `src/components/ProfileDialog.tsx` (or extend existing dialog pattern)

#### Edge Cases & Risks
- **Concurrent modification**: if user toggles mods while profile is active, those changes auto-sync to the profile. This is intentional — profile is live, not frozen. User must explicitly "save as new" to fork.
- **Profile from ad-hoc state**: creating a profile while in ad-hoc mode should snapshot current state. After creation, user is now in profile mode (profile active).
- **Mod import while profile active**: new mod added to game, also added to profile's `mod_states` as disabled. If we forget this, the mod won't appear in the profile on next restore.
- **Profile name collisions**: enforce unique names per game (not globally). Validate on create/rename.
- **Config size**: 10 profiles × 200 mods × ~100 bytes per `ModState` = ~200KB. Still fine for JSON. Monitor; migrate to SQLite if profiles become large.
- **Backup/restore compatibility**: backups are full `AppConfig` snapshots, so profiles are automatically included. Restoring a pre-profile backup (v2 config) migrates cleanly.
- **Switching away from ad-hoc**: if user creates a profile after hours of ad-hoc tweaking, those tweaks become the profile's initial state. Edge case: user expected a "blank" profile. Solution: provide both "Save Current as Profile" and "Create Empty Profile" options.

#### Hardening Analysis
- **Atomicity**: `apply_profile` must be atomic — if deploy fails halfway, config should roll back to previous state. Use a pre-save backup: clone current config, attempt deploy, restore clone on failure.
- **Orphan profiles**: after deleting a mod, check all profiles for references to that mod ID and clean them up. Do this in `delete_mod`.
- **UX trap**: if user doesn't realize a profile is active and starts toggling mods, they might corrupt their "golden" profile. Mitigation: show active profile name prominently in the mod list header (colored badge). Optional: add a "lock profile" toggle that makes it read-only until unlocked.
- **Migration safety**: v2 → v3 migration adds empty `profiles: vec![]` and `active_profile_id: None`. Existing behavior is fully preserved.

#### Implementation Phases
1. Add `Profile` and `ModState` structs, bump config to v3, write migration
2. Add `save_config` Tauri command
3. Add `apply_profile` Tauri command with atomic deploy
4. Build `ProfileDialog` component (list, create, delete, rename)
5. Add profile dropdown to `ModList` header
6. Wire mod add/delete to sync with active profile
7. Test: create profile, switch, delete mod, verify profile consistency

---

### 3. Bulk Select & Batch Operations

#### Summary
Add checkbox-based multi-select to the mod list, enabling batch enable, disable, delete, and reorder operations.

#### Why
Deploy All / Purge All are the only bulk actions. User with 60 mods who wants to disable 15 specific mods must toggle each individually. "Select all on this page" / "Select all matching filter" patterns make this fast.

#### Current State
- `Deploy All` and `Purge All` are the only bulk operations — they target ALL enabled mods
- No selection state exists
- `toggle_mod` and `delete_mod` operate on single mod IDs

#### Design

**Frontend state in `ModList.tsx`:**

```
selectedModIds: Set<string>
lastClickedIndex: number | null  // for shift-click range select
```

**Selection UX:**
- Checkbox on each `ModCard` (left of drag handle for load-order games, left of mod name otherwise)
- Click checkbox: toggle single selection
- Shift+click checkbox: range select from last clicked
- Ctrl/Cmd+click checkbox: toggle single without affecting others
- "Select all" checkbox in toolbar (with count: "8 selected")
- Clicking the mod card itself does NOT select — only checkbox interaction selects. This prevents accidental selection during drag or toggle.

**Batch actions toolbar:**
Appears when `selectedModIds.size > 0`, between the main toolbar and the mod list:
- "Enable selected" button
- "Disable selected" button
- "Delete selected" button (red, with confirmation dialog)
- Selection count: "8 mods selected"

**Backend:**

Two approaches:
1. Loop `toggle_mod`/`delete_mod` N times from frontend (simple, but N round trips)
2. New batch commands (fewer round trips)

**Decision: single new command `batch_toggle(game_id, mod_ids, enabled)` and `batch_delete(game_id, mod_ids)`.**

`batch_toggle`:
- Iterates mod_ids, checks conflicts per mod (must handle: toggling mod A + mod B at same time — if A and B conflict, which wins? Error on first conflict, or toggle what we can?)
- **Policy: partial success with error report.** Enable as many as conflict rules allow. Return list of mods that failed and why. Show toast: "12 enabled, 3 blocked by conflicts: ModX conflicts with ModY".

`batch_delete`:
- Removes symlinks for enabled mods in the batch, then removes entries
- Confirmation dialog required: "Delete 8 mods? This cannot be undone. Mod files in library will be kept."

#### Integration Points
- `ModList.tsx`: selection state, batch toolbar, checkbox rendering
- `ModCard.tsx`: add checkbox prop, render conditionally
- `lib.rs`: add `batch_toggle` and `batch_delete` commands
- `conflicts.rs`: `check_enable_conflict` already works per-mod; batch mode needs awareness of the batch (a mod that conflicts with another mod in the same batch should be flagged)

#### Edge Cases & Risks
- **Conflict within batch**: enabling ModA and ModB in same batch, where ModA conflicts with ModB. Current `check_enable_conflict` checks against already-enabled mods — it won't see ModB if ModB is also being enabled. Solution: in `batch_toggle`, build a temporary "would-be-enabled" set that includes both already-enabled mods AND the batch mods being enabled, then check each batch mod against that expanded set.
- **Shift+click with filters active**: range select should only select visible (filtered) mods, not mods hidden by search. Otherwise user selects mods they can't see.
- **Delete with active profile**: removing mod from game must also remove from all profiles' `mod_states`.
- **Large batch performance**: 200 mods × symlink creation is already handled by `deploy_all`. Batch toggle of 200 mods is same cost. No new perf concern.
- **Undo**: no undo for batch operations. Mitigation: confirmation dialog for destructive ops (delete), clear toast feedback for toggles showing what happened.

#### Hardening Analysis
- **Selection persistence**: clear selection when switching games, changing sort order, or applying a filter. Stale selection across these transitions could target wrong mods.
- **Drag + select interaction**: drag handle must NOT trigger checkbox. Checkbox click must NOT trigger drag. Use `event.stopPropagation()` on checkbox and explicit drag Handle component separation.
- **Batch toggle partial failure UX**: return structured errors: `{ failed: [{modId, modName, reason, conflictingMods: [...]}], succeeded: number }`. Frontend renders specific error toast with mod names.

#### Implementation Phases
1. Add selection state + checkbox to `ModCard`
2. Add batch toolbar to `ModList`
3. Add `batch_toggle` backend command with conflict awareness
4. Add `batch_delete` backend command
5. Handle shift+click range selection
6. Handle select-all + filter interaction

---

## Tier 2 — High Value, Large Impact

---

### 4. Mod Metadata

#### Summary
Extend `ModEntry` with structured metadata: version, author, description, source URL, category tags. Extract what is possible from archive filenames and readme files; allow manual editing.

#### Why
Currently a mod is just a name + archive source + file list. No way to know if a mod is outdated, who made it, what it does, or where to get updates. This blocks update checking (Feature 8), categories (Feature 11), and general usability.

#### Current State
```rust
struct ModEntry {
    id: String,
    name: String,
    archive_source: String,
    enabled: bool,
    priority: u32,
    installed_files: Vec<String>,
}
```

No version, no author, no description, no source URL.

#### Design

**Extended `ModEntry` (config v4, or fold into v3 with profiles):**

```rust
struct ModEntry {
    // existing fields unchanged
    id: String,
    name: String,
    archive_source: String,
    enabled: bool,
    priority: u32,
    installed_files: Vec<String>,

    // new metadata fields (all optional)
    version: Option<String>,         // e.g. "1.4.2"
    author: Option<String>,          // e.g. "cdprojektred"
    description: Option<String>,     // short text, max 500 chars
    source_url: Option<String>,      // e.g. "https://www.nexusmods.com/witcher3/mods/1234"
    category: Option<String>,        // e.g. "Gameplay", "Graphics", "UI", "Patches", "Overhaul", "Other"
    tags: Vec<String>,               // freeform tags for filtering
    installed_at: String,            // ISO 8601, set on import
    updated_at: Option<String>,      // ISO 8601, set on metadata edit
}
```

**Metadata extraction on import:**

When importing an archive, attempt to auto-extract:
1. **Name**: already extracted from archive filename (strip version-like suffixes: "ModName-v1.4.2" → name="ModName", version="1.4.2")
2. **Version**: regex match common patterns in filename: `v(\d+\.\d+\.\d+)`, `-(\d+\.\d+)-`, `_(\d+_\d+_\d+)`
3. **Readme scan**: if archive contains a file matching `readme*`, `README*`, `about*.txt`, extract first 500 chars as description candidate. Show in import dialog for user to accept/edit.

**Manual editing:**

New dialog or inline edit on `ModCard`:
- Click mod name or an "info" icon → opens `ModInfoDialog`
- Editable fields: name, version, author, description, source URL, category, tags
- "Save" updates `ModEntry` in config via `save_config`
- "Open Source URL" button (external link, opens in browser via `tauri::api::shell::open`)

**Display on ModCard:**

Add a second line under mod name showing:
- Version badge (if set): "v1.4.2" in mono font
- Author (if set): "by cdprojektred"
- Category tag (if set): colored badge
- On hover: truncated description tooltip

ModCard height increases slightly (~8-12px). Acceptable for the information gain.

#### Integration Points
- `config.rs`: extend `ModEntry` struct, bump config version, migration fills new fields as `None`
- `archive.rs`: add version extraction regex, readme scan in `import_mod`
- `ImportModDialog.tsx`: show extracted metadata preview, let user edit before confirming import
- `ModCard.tsx`: render metadata line (if any fields are set), add info button
- New `ModInfoDialog.tsx`: edit metadata form
- `App.tsx`: new dialog handler, `open_url` handler
- `lib.rs`: add `open_url` command (or use Tauri shell plugin)

#### Edge Cases & Risks
- **Config migration**: all existing mods get `None` for all new fields. No data loss.
- **Version parsing false positives**: "ModPack-v2" → version="2" is wrong if mod name is "ModPack v2". Mitigation: show extracted metadata in import dialog, let user correct it. Never auto-assign without user seeing it.
- **Readme extraction**: large readme files. Only extract first 500 chars. Binary readme detection: if file contains null bytes, skip. Encoding: try UTF-8, fall back to latin1.
- **Category enum vs freeform**: start with a fixed list dropdown (Gameplay, Graphics, UI, Audio, Patches, Overhaul, Utilities, Other) but allow custom entry. This balances structure with flexibility.
- **Source URL validation**: must start with `https://`. Warn on `http://`. Reject `file://` and `javascript:`.

#### Hardening Analysis
- **Config size**: 200 mods × ~1KB metadata each = ~200KB. Still fine for JSON. Monitor.
- **Search integration**: feature 1's search should also match against author, description, and tags. Update search logic when metadata lands.
- **Backup compatibility**: metadata is in config, so backups include it automatically.
- **Import flow UX**: don't block import on metadata entry. Let user import quickly with auto-extracted metadata, refine later. "Import" button is primary; metadata editing is secondary.

#### Implementation Phases
1. Extend `ModEntry` struct, bump config, write migration
2. Add version regex extraction in `archive.rs`
3. Add readme scan in `archive.rs`
4. Build `ModInfoDialog` component
5. Update `ModCard` to show metadata line
6. Update `ImportModDialog` to show extracted metadata preview
7. Integrate metadata fields into search (Feature 1 follow-up)

---

### 5. Game Auto-Detection

#### Summary
Scan common library paths (Steam, Lutris, Heroic, Bottles) for supported games and offer a one-click "Add Found Games" flow, replacing the fully manual path entry.

#### Why
Current flow: user must know exact install paths. For Proton games (SoD2), the path must be the Windows `exe` location inside the prefix, not the Steam library path. This is confusing and error-prone. Auto-detection removes the biggest onboarding friction.

#### Current State
- `AddGameDialog` asks for game type + display name + install path (manual)
- No scanning logic exists
- README explicitly says "No: auto-detection"

#### Design

**Scanning strategy:**

The backend scans known locations on demand (user clicks "Scan for games" in AddGameDialog or sidebar):

**Steam:**
- Parse `~/.steam/steam/steamapps/libraryfolders.vdf` for library paths
- For each library, scan `steamapps/common/` for matching game directory names
- Match against a known-games registry: `{ "The Witcher 3": { steamAppId: 292030, dirPatterns: ["The Witcher 3", "The Witcher 3 Wild Hunt"] }, ... }`
- For Proton games, resolve the prefix path from `steamapps/compatdata/<appid>/pfx/`

**Lutris:**
- Parse `~/.config/lutris/games/*.yml` for game configs
- Extract `game.exe` path and working directory

**Heroic:**
- Parse `~/.config/heroic/gog_store/installed.json` and `~/.config/heroic/legendary/installed.json`
- Extract install paths

**Bottles:**
- Parse `~/.local/share/bottles/bottles/*.yml` for bottle configs
- Extract drive paths and executables

**Scan architecture:**

New Rust module: `src-tauri/src/detection.rs`

```rust
struct DetectedGame {
    game_type: String,          // "witcher3", "sod2"
    display_name: String,       // pre-filled name
    install_path: PathBuf,      // verified existing path
    source: DetectionSource,    // "steam", "lutris", "heroic", "bottles"
    source_detail: String,      // "Steam Library on /mnt/games"
}
```

Detection is done by a **game registry** — a static list of known games with patterns for each launcher:

```rust
struct KnownGame {
    game_type: String,
    steam_app_id: Option<u32>,
    steam_dir_names: Vec<String>,      // directory name patterns in steamapps/common/
    lutris_slugs: Vec<String>,         // Lutris game slugs
    heroic_app_names: Vec<String>,     // Heroic game identifiers
    is_proton: bool,                   // needs prefix path resolution
    proton_app_id: Option<u32>,        // if Proton, the Steam App ID for prefix
    relative_exe_path: Option<String>, // path within install dir to verify
}
```

**Frontend:**

`AddGameDialog` changes:
- Add a "Scan for Games" button at the top
- On click, calls `scan_for_games` backend command (may take 2-5 seconds)
- Shows progress: "Scanning Steam...", "Scanning Lutris..."
- Results shown as a list: game icon, name, detection source, install path. Each has an "Add" button.
- User can still manually add a game below the scan results (existing flow preserved)

**Manual path hints improved:**
- For each game type, show a "Where to find this" hint with common paths and expected folder contents
- For Proton games, explain the path structure with a diagram

#### Integration Points
- New file: `src-tauri/src/detection.rs`
- `src-tauri/src/games/known_games.rs` — the game registry data (static, compiled)
- `lib.rs`: add `scan_for_games` Tauri command
- `AddGameDialog.tsx`: scan button, results list
- `types.ts`: add `DetectedGame` interface

#### Edge Cases & Risks
- **Multiple installs**: same game installed in multiple libraries (Steam + Lutris). Show both, let user pick.
- **Library on external drives**: libraryfolders.vdf handles this. Mount check: if library path doesn't exist, skip with warning.
- **Steam library on NTFS**: common on dual-boot. Symlinks may not work on NTFS. Detect filesystem type and warn: "Mod symlinks may not work on NTFS drives. Consider moving the game to a Linux-native filesystem."
- **Lutris/Heroic not installed**: gracefully skip that source (no error, just nothing found there).
- **Bottles with custom paths**: bottles can be anywhere. The default location is the best-effort scan.
- **Proton prefix not yet created**: SoD2 needs the prefix to exist (first launch creates it). Detect and warn: "Launch the game once through Steam before adding it."

#### Hardening Analysis
- **Performance**: VDF parsing + YAML parsing + filesystem traversal = 2-5 seconds per scan. Cache results for the session (don't rescan on every dialog open). Add a "Rescan" button if user installed a game while manager is open.
- **False positives**: a directory named "The Witcher 3" might not be the game. Verify by checking for the game's known executable or marker file (`bin/x64/witcher3.exe` or `bin/x64_vk/witcher3.exe`).
- **Privacy**: scanning reads file paths but sends nothing off-machine. No telemetry.
- **Steam Deck**: `~/.steam/steam` is a symlink to `~/.local/share/Steam` on Deck. Standard Steam paths still work.
- **Game registry maintainability**: keep it as a static Rust `Vec<KnownGame>` or `const` array. Adding a game = adding an entry to this list. This is Feature 6 (declarative definitions), but for now hardcoded is acceptable.

#### Implementation Phases
1. Build `detection.rs` with Steam scanner
2. Add Lutris scanner
3. Add Heroic scanner
4. Add Bottles scanner
5. Build game registry with Witcher 3 + SoD2 entries
6. Add `scan_for_games` Tauri command
7. Update `AddGameDialog` with scan button and results list
8. Add filesystem type check (NTFS warning)

---

### 6. Declarative Game Definitions

#### Summary
Move game-specific logic (mod directory, save directory, validation rules, deploy behavior, conflict mode, load order support) from hardcoded Rust structs to a JSON schema. Ship a `games/` directory with built-in definitions; allow users to add custom ones.

#### Why
Currently adding a game means writing Rust code, implementing the `GameTrait`, and recompiling. This blocks community contributions and makes the app rigid. With 2 games supported, every new game is a PR and a release. A declarative system means 50 games = 50 JSON files.

#### Current State
- `src-tauri/src/games/mod.rs`: `GameTrait` with methods: `mod_dir()`, `save_dir()`, `validate_archive()`, `derive_installed_files()`, `post_deploy()`, `supports_load_order()`, `conflict_mode()`
- `src-tauri/src/games/witcher3.rs`: Witcher 3 implementation (70 lines)
- `src-tauri/src/games/sod2.rs`: SoD2 implementation (80 lines)
- Game selection is by string match: `game.game_type == "witcher3"` → instantiate `Witcher3`

#### Design

**Game definition schema (JSON):**

```json
{
  "type": "witcher3",
  "display_name": "The Witcher 3: Wild Hunt",
  "engine": "REDengine 3",
  "icon": "sword",
  "mod_directory": {
    "relative_to": "install_path",
    "path": "Mods"
  },
  "save_directory": {
    "relative_to": "install_path",
    "path": "Documents/The Witcher 3/gamesaves"
  },
  "save_pattern": "*.sav",
  "validation": {
    "mode": "contains_subdirectory",
    "value": "content"
  },
  "install_files": {
    "mode": "relative_paths",
    "strip_prefix": null
  },
  "load_order": {
    "supported": true,
    "config_file": {
      "relative_to": "install_path",
      "path": "Documents/The Witcher 3/modssettings",
      "format": "ini_section",
      "section": "mods"
    }
  },
  "conflicts": "warn",
  "launch": {
    "steam_app_id": 292030,
    "known_executables": [
      "bin/x64/witcher3.exe",
      "bin/x64_vk/witcher3.exe"
    ]
  },
  "detection": {
    "steam": { "app_id": 292030, "dir_names": ["The Witcher 3", "The Witcher 3 Wild Hunt"] },
    "gog": { "product_id": 1207664643 },
    "registry_paths": []
  },
  "proton": null
}
```

**Schema design principles:**
- Every field that varies per game is in the JSON
- Fields that require custom code (`post_deploy` for Witcher 3's `mods.settings`) are handled by **script hooks** — if a game needs custom logic, it specifies a `hook` field referencing a known hook type
- Hook types are the escape hatch: "if `load_order.config_file` exists, use built-in INI writer. If a game needs something else, add a new hook type to Rust."

**Hook registry (Rust):**

Keep the trait system but make it optional. A game definition can specify `hook: "witcher3_load_order"` or `hook: null`. Hooks are registered in a `HashMap<String, Box<dyn GameHook>>`:

```rust
trait GameHook {
    fn post_deploy(&self, game: &GameEntry, mods: &[ModEntry], library_base: &Path) -> Result<()>;
    fn validate_archive(&self, archive_root: &Path) -> Result<Vec<String>>;
    fn derive_installed_files(&self, archive_root: &Path, mod_name: &str) -> Result<Vec<String>>;
    fn resolve_save_dir(&self, install_path: &Path) -> Option<PathBuf>;
    fn resolve_mod_dir(&self, install_path: &Path, proton_prefix: Option<&Path>) -> Result<PathBuf>;
}
```

Built-in hooks:
- `witcher3_load_order`: writes `mods.settings` INI
- `sod2_proton`: resolves Proton prefix path, flattens .pak names
- `generic`: default implementation — straight path join, relative file tracking, no post_deploy

New games that don't need custom hooks use `"hook": null` → the `generic` hook handles everything from the JSON definition alone.

**Installation:**
- Built-in definitions: `src-tauri/games/definitions/*.json` — compiled into the binary via `tauri-build` resource embedding
- User definitions: `~/.config/linux-mod-manager/games/*.json` — loaded at startup, merged with built-ins (user defs override built-ins of same `type`)

**Frontend changes:**
- `AddGameDialog` game type dropdown populated dynamically from loaded definitions
- Support status: built-in definitions are "verified" or "provisional" (set in JSON). User definitions are always "community" with a different badge.
- Unknown games (from old configs where the definition was removed) show as "legacy" with a warning.

#### Integration Points
- `src-tauri/src/games/`: refactor to hook registry + JSON loader
- `src-tauri/games/definitions/`: new directory with built-in JSON files
- `lib.rs`: load game definitions on startup, expose `get_game_definitions` command
- `AddGameDialog.tsx`: dynamic game type list
- `Sidebar.tsx`: show support status from definition
- `config.rs`: no changes (game type is still a string)

#### Edge Cases & Risks
- **User definition conflicts with built-in**: user definition with same `type` overrides. Built-in update that adds new fields — user definition doesn't have them — use built-in defaults for missing fields.
- **Definition validation**: load JSON, validate against schema at startup. Reject invalid definitions with clear error messages (don't crash, don't silently ignore).
- **Hook not found**: if a definition references `hook: "skyrim_se"` but no such hook is compiled in, fall back to `generic` and warn.
- **Circular override**: user defines `witcher3` with different mod directory → their paths override. This is intentional flexibility.
- **Schema versioning**: include a `schema_version: 1` field in each definition. When schema evolves, migrate on load or reject with upgrade instructions.
- **Security**: user-provided JSON could contain path traversal strings. Sanitize all path fields on load: reject `..`, absolute paths, and null bytes.

#### Hardening Analysis
- **Migration from Rust to JSON**: existing Witcher 3 and SoD2 definitions must be 1:1 equivalent after migration. Write comparison tests: for each game, load JSON definition, run through generic+hook, assert output matches current hardcoded output.
- **Performance**: 50 game definitions × 2KB each = 100KB. Load once at startup, cache in app state.
- **Hot reload**: definitions loaded at startup. Adding a user definition requires restart. Acceptable for v1; add "Reload definitions" button later.
- **Discoverability**: how does user know they can add custom games? Menu bar entry: "Help > Custom Game Definitions" opens docs. Or a "Community definitions" button that points to a GitHub repo.
- **Dependency: this unblocks Features 7, 8, and 9**: more games (7) just need JSON files. Update checking (8) can look at `source_url` patterns. Auto-detection (5) uses `detection` block.

#### Implementation Phases
1. Design JSON schema, document all fields
2. Implement schema validation in Rust (serde + JSON Schema or manual validation)
3. Implement `generic` hook
4. Implement `witcher3_load_order` hook
5. Implement `sod2_proton` hook
6. Port Witcher 3 definition to JSON, verify 1:1 behavior
7. Port SoD2 definition to JSON, verify 1:1 behavior
8. Add definition loading + merge logic
9. Update `AddGameDialog` to use dynamic game list
10. Write schema documentation for community contributors

---

### 7. Expanded Game Support (via Declarative Definitions)

#### Summary
After Feature 6 lands, add built-in definitions for popular moddable games. Each definition is a JSON file + optional hook if the game needs custom deploy logic.

#### Target Games (ordered by modding community size)

**No custom hook needed (generic hook only):**
| Game | Mod Type | Load Order | Difficulty |
|---|---|---|---|
| Cyberpunk 2077 | Folder-based (`archive/pc/mod/`) | No (load order via file naming prefix) | Easy |
| Baldur's Gate 3 | `.pak` files in `Mods/` | Yes (`modsettings.lsx` XML) | Medium (needs XML hook) |
| Stardew Valley | SMAPI/xnb replacement | No (SMAPI handles it) | Easy |
| Valheim | BepInEx plugins | No | Easy |
| Subnautica | QMods/BepInEx | No | Easy |
| Red Dead Redemption 2 | `.asi`/`.dll` + `lml/` folder | No | Easy |
| Kingdom Come: Deliverance | `.pak` in `Mods/` | No | Easy |
| Dyson Sphere Program | BepInEx plugins | No | Easy |
| Risk of Rain 2 | BepInEx plugins | No | Easy |
| Terraria | tModLoader | No (separate launcher) | Medium |
| Mount & Blade II: Bannerlord | Module folders | Yes (XML-based) | Medium |

**Needs custom hook:**
| Game | Hook Required | Complexity |
|---|---|---|
| Skyrim SE/AE | ESL/ESP management, LOOT sorting, SKSE detection | High |
| Fallout 4 | ESL/ESP management, LOOT sorting, F4SE detection | High |
| Skyrim (LE) | Same as SE but different paths | High |
| Fallout: New Vegas | Same pattern | High |
| Cyberpunk 2077 | REDmod integration, CET detection | Medium |
| Baldur's Gate 3 | `modsettings.lsx` LSX XML format | Medium |
| Starfield | ESL/ESP + `StarfieldCustom.ini` | High |
| Minecraft (Java) | Fabric/Forge/Quilt loader detection, mods folder | Medium |
| Sims 4 | Package files, script mods, resource.cfg | Medium |

#### Prioritization

**Wave 1** (generic hook only, high demand, low effort):
1. Cyberpunk 2077
2. Valheim
3. Stardew Valley
4. Kingdom Come: Deliverance

**Wave 2** (needs one new hook type):
5. Baldur's Gate 3 (LSX hook)
6. Red Dead Redemption 2

**Wave 3** (Bethesda games — need ESL/ESP plugin system, Feature 12):
7. Skyrim SE/AE
8. Fallout 4

#### Design Per Game (Highlights)

**Cyberpunk 2077:**
- Mod directory: `<game>/archive/pc/mod/`
- Mod format: folder containing files (no subfolder validation — any structure works)
- Load order: determined by file name prefix (A_, B_, etc.). Not managed by app yet; warn user in documentation.
- REDmod detection: check for `tools/redmod/bin/redmod.exe`. If present, show REDmod toggle.
- Proton: yes (GOG and Steam versions). Prefix at `steamapps/compatdata/1091500/`.

**Baldur's Gate 3:**
- Mod directory: `<game>/Data/Mods/` — wait, actually it's `%LOCALAPPDATA%/Larian Studios/Baldur's Gate 3/Mods/` on Windows, which on Proton is inside the prefix.
- Save directory: inside prefix at `.../Larian Studios/Baldur's Gate 3/PlayerProfiles/<profile>/Savegames`
- Mod format: `.pak` files
- Load order: `modsettings.lsx` — LSX is custom XML. Needs a new hook that parses and rewrites the `<children>` node in `<node id="Mods">`. This is parse-modify-serialize, not simple template.
- Multiplayer consideration: different profiles may have different mod sets. Warn on mismatch.

**Valheim:**
- Mod directory: `<game>/BepInEx/plugins/`
- Mod format: DLL files in archive
- BepInEx detection: check for `BepInEx/core/BepInEx.dll`
- No load order
- Dedicated server: users may want to sync mods with server. Out of scope for v1 but note it.

#### Integration Points
- New JSON files in `src-tauri/games/definitions/`
- New hooks as needed: `bg3_lsx`, `cyberpunk_redmod` (if not generic)
- Detection registry updates in `detection.rs`

#### Edge Cases & Risks
- **Proton game path variety**: GOG Galaxy, EGS via Heroic, Lutris — all have different prefix locations. The detection system (Feature 5) must handle all of them.
- **Game updates breaking paths**: game update moving directories. Detection should use Steam App ID, not hardcoded paths. App IDs are stable.
- **Bethesda edition soup**: Skyrim has LE, SE, AE, VR. Fallout 4 has standard and VR. Each is a separate definition with different paths and App IDs.
- **Multiple valid mod formats**: Cyberpunk can mod via `archive/pc/mod/` OR REDmod `mods/<name>/`. Two deployment strategies for one game. Handle with a `mod_format` option in the definition or a separate game type entry.

#### Hardening Analysis
- **Testing without owning the game**: use test fixtures — a directory structure mimicking the game with placeholder files. Integration tests verify mod deployment, conflict detection, and load order for each game definition.
- **Definition quality bar**: built-in definitions must be tested against real game installs before shipping. "Provisional" tag for community-contributed definitions that haven't been verified by maintainers.
- **User expectations**: more games = more support burden. Each game definition carries an implicit promise. Be clear about support levels: "Verified" (tested by maintainer), "Community" (user-contributed, works for most), "Provisional" (built-in but lightly tested).

---

## Tier 3 — Competitive Features

---

### 8. Mod Update Checking

#### Summary
Compare installed mod versions against latest available versions. Check via Nexus Mods API, GitHub releases, or manual version URLs. Show update availability badges on mod cards.

#### Why
Users with 100+ mods cannot manually check each mod's source for updates. This is the #1 reason users eventually switch to Vortex/MO2 despite their complexity.

#### Current State
- No version tracking (addressed by Feature 4)
- No external API integration
- README says "No: Nexus API"

#### Design

**Update sources (priority order):**

1. **Nexus Mods API v1**: requires user-provided API key. Rate limited (2,500 req/day for free users, more for premium). GET `/v1/games/{game_domain}/mods/{mod_id}.json` returns latest version.
2. **GitHub Releases API**: no auth required for public repos. Rate limited (60 req/hour unauthenticated, 5000 with token). GET `/repos/{owner}/{repo}/releases/latest` returns tag_name.
3. **Manual check URL**: user provides any URL. App fetches it and regex-matches a version pattern. Fallback for mods on Patreon, ModDB, or personal sites.

**Per-mod configuration (extends Feature 4 metadata):**

```rust
struct UpdateSource {
    source_type: UpdateSourceType,  // "nexus", "github", "manual"
    // Nexus
    nexus_game_domain: Option<String>,  // e.g. "witcher3"
    nexus_mod_id: Option<u32>,          // e.g. 1234
    // GitHub
    github_repo: Option<String>,        // e.g. "owner/repo"
    // Manual
    manual_url: Option<String>,
    version_regex: Option<String>,      // e.g. "v(\\d+\\.\\d+\\.\\d+)"
}
```

**Update check flow:**

1. User clicks "Check for Updates" in toolbar (per-game) or "Check All" (global)
2. Backend iterates mods with `UpdateSource` configured
3. For each source, fetch latest version string via HTTP
4. Compare with `ModEntry.version` using semver or string comparison
5. Return results: `{ mod_id, current_version, latest_version, update_url, is_update }`
6. Frontend shows update badge on mod cards: "v1.4.2 → v1.5.0"
7. "Update All" button: opens each update URL in browser (app doesn't download mods — Feature 10)

**Rate limiting & caching:**
- Cache results for 1 hour (configurable)
- Stagger requests: 200ms delay between checks to stay under rate limits
- Show progress: "Checking 45 mods... (12/45)"
- Respect Nexus API rate limit headers (`x-rl-hourly-limit`, `x-rl-hourly-remaining`)

**Backend:**

New module: `src-tauri/src/update_check.rs`

```rust
struct UpdateResult {
    mod_id: String,
    mod_name: String,
    current_version: Option<String>,
    latest_version: String,
    source_url: String,
    is_update_available: bool,
    error: Option<String>,  // null if check succeeded
}
```

New Tauri command: `check_for_updates(game_id: Option<String>)` — if `game_id` is Some, check only that game's mods; if None, check all.

**Frontend:**

- "Check for Updates" button in `ModList` toolbar (with refresh icon)
- Update badge on `ModCard`: green if up-to-date, amber if update available, grey if no version/source configured
- Click badge → opens update URL in browser
- "Update All Available" button (shown when updates are found) → opens all update URLs sequentially
- New `UpdateCheckDialog`: progress bar + result list (which mods were checked, which have updates, any errors)

#### Integration Points
- `config.rs`: add `UpdateSource` to `ModEntry`
- New file: `src-tauri/src/update_check.rs`
- `lib.rs`: add `check_for_updates` command, HTTP client setup (reqwest)
- `Cargo.toml`: add `reqwest` with `rustls-tls` feature (no OpenSSL dependency)
- `ModCard.tsx`: update badge
- `ModInfoDialog.tsx`: update source configuration UI (new tab or section)
- `ModList.tsx`: "Check for Updates" button
- New `UpdateCheckDialog.tsx`

#### Edge Cases & Risks
- **No version set**: if `ModEntry.version` is None, can't compare. Show "Version unknown — unable to check" status. Encourage user to set version in ModInfoDialog.
- **Nexus API key management**: store API key in config. Mask in UI. "Get API Key" link to Nexus. Validate key on entry (test request).
- **Nexus API changes**: v1 is deprecated in favor of v2 (GraphQL). Build against v2 from the start.
- **GitHub rate limiting**: 60 req/hour unauthenticated = checking 60 mods per hour max. Encourage GitHub token setup. Show rate limit status.
- **Semver comparison**: many mod versions aren't semver ("1.4.2hotfix", "1.4.2a", "Final"). Use lenient comparison: split on dots, compare numerically where possible, fall back to string comparison. Be conservative: if comparison is ambiguous, don't claim "update available," show "check manually."
- **Network errors**: no internet, timeout, DNS failure. Don't block the app. Show error toast and let user retry.
- **HTML pages as "manual URL"**: user sets manual URL to a forum thread. Regex can't find version in HTML. Mitigation: show fetched text snippet in ModInfoDialog so user can refine their regex.

#### Hardening Analysis
- **Dependency: requires Feature 4 (metadata)**: version field and source URL must exist before update checking makes sense.
- **Privacy**: update checking makes outbound HTTP requests. Users must opt in (enter API key, configure sources). No automatic phoning home.
- **Error resilience**: one mod's check failing must not abort the batch. Catch per-mod, report errors in results, continue.
- **Nexus premium vs free**: premium users get 10x rate limit. Detect from API response headers. Adjust stagger delay accordingly.
- **Caching strategy**: cache in memory (HashMap) during session. On app restart, cache is empty. First check after launch is slow; subsequent checks are fast. This is acceptable.

#### Implementation Phases
1. Add `reqwest` to Cargo.toml
2. Implement GitHub releases checker
3. Implement Nexus Mods API checker
4. Implement manual URL + regex checker
5. Add `UpdateSource` to `ModEntry`, config migration
6. Build `check_for_updates` command with caching + rate limiting
7. Build `UpdateCheckDialog` frontend
8. Add update badges to `ModCard`
9. Add update source configuration to `ModInfoDialog`

---

### 9. Dependency Tracking

#### Summary
Define relationships between mods: requires, conflicts, recommends, loads-before, loads-after. Enforce on enable. Goes beyond file-level conflict detection.

#### Why
File conflict detection only catches overlapping files. Real mod ecosystems have semantic dependencies: "Mod B requires Mod A's assets," "Mod C is incompatible with Mod D (not files, but logic)," "Patch E must load after Mod F." Without dependency tracking, users debug broken mod setups by trial and error.

#### Current State
- `conflicts.rs` detects file-level overlaps only
- No concept of mod relationships
- `check_enable_conflict` only checks file overlaps against enabled mods

#### Design

**Relationship types:**

```rust
enum ModRelation {
    Requires,        // Hard: this mod won't work without the target
    Conflicts,       // Hard: these mods cannot both be enabled
    Recommends,      // Soft: suggests but doesn't require
    LoadsAfter,      // Priority constraint: this must load after target
    LoadsBefore,     // Priority constraint: this must load before target
    Provides,        // This mod provides the functionality of another mod (alternative)
}
```

**Data model:**

Add to `ModEntry`:
```rust
relationships: Vec<ModRelationEntry>,

struct ModRelationEntry {
    target_mod_id: Option<String>,   // specific mod ID (preferred)
    target_mod_name: Option<String>,  // fallback: match by name (for mods not yet imported)
    relation_type: ModRelation,
    note: Option<String>,            // e.g. "Requires v2.0+ of Base Mod"
}
```

**Dependency graph:**

On enabling a mod, build a dependency graph for the game and check:
1. **Hard requirements**: all `Requires` targets must be enabled (or enable them automatically, with user confirmation).
2. **Hard conflicts**: no `Conflicts` targets can be enabled.
3. **Soft recommendations**: warn but don't block.
4. **Load order constraints**: `LoadsAfter`/`LoadsBefore` constrain priority values. If violated, auto-adjust priority or warn.
5. **Circular dependencies**: detect and reject (can't have A requires B requires A).
6. **Missing targets**: if `target_mod_name` is set but no mod with that name is imported, warn: "Requires 'Base Mod' which is not installed."

**Enabling flow with dependencies:**

1. User toggles Mod A on
2. Backend builds dependency tree from Mod A outward
3. If `Requires` targets exist and are disabled: "Enabling 'Mod A' requires: 'Base Mod', 'Shared Assets'. Enable them too?" [Enable All] [Cancel]
4. If `Conflicts` targets are enabled: "Cannot enable 'Mod A': it conflicts with 'Old Overhaul'. Disable 'Old Overhaul' first?" [Disable & Enable] [Cancel]
5. If `LoadsAfter` constraint violated: auto-fix priority silently (or warn if auto-fix is ambiguous)
6. File-level conflict check still runs

**Defining relationships:**

Option A: **Manual entry** in `ModInfoDialog` — user fills in relationship table. Accurate but tedious.

Option B: **Metadata file in archive** — if mod archive contains `skuld-manifest.json` or `mod-manifest.json`, parse relationships from it. Standardize the format, encourage mod authors to include it.

Option C: **Community database** — a curated repository of relationship data per game, downloaded by the app. Similar to LOOT's masterlist.

**Recommendation: Option B as primary (manifest file), Option A as fallback (manual entry).** Option C (community DB) can be added later as a periodic download.

**Manifest format (shipped in mod archives):**

```json
{
  "schema_version": 1,
  "name": "My Mod",
  "version": "1.4.2",
  "author": "modder123",
  "description": "...",
  "game": "witcher3",
  "relationships": {
    "requires": [
      { "name": "Community Patch - Base", "version_min": "1.0" }
    ],
    "conflicts": [
      { "name": "Old Overhaul" }
    ],
    "loads_after": [
      { "name": "Community Patch - Base" }
    ]
  }
}
```

On import, if `skuld-manifest.json` is found in the archive root, auto-populate metadata AND relationships.

**Frontend:**

- `ModCard`: show relationship badges: "Requires 2 mods", "Conflicts with 1", "2 mods require this"
- `ModInfoDialog`: relationship editor tab — table with add/remove rows. Each row: relation type dropdown, target mod selector (searchable), optional note.
- Enable flow: dependency resolution dialog shown before enabling. Shows tree of affected mods.

**Backend:**

New module: `src-tauri/src/dependencies.rs`

```rust
fn resolve_dependencies(game: &GameEntry, mod_id: &str, action: EnableAction) -> DependencyResult {
    // BFS from mod_id following Requires/Conflicts
    // Returns: list of mods to auto-enable, list of mods that must be disabled, list of warnings
}
```

#### Integration Points
- `config.rs`: add `relationships` to `ModEntry`, bump config version
- New file: `src-tauri/src/dependencies.rs`
- `archive.rs`: parse `skuld-manifest.json` on import
- `lib.rs`: add `resolve_dependencies` command (run before `toggle_mod`), or fold into `toggle_mod`
- `ModCard.tsx`: relationship badges
- `ModInfoDialog.tsx`: relationship editor
- New `DependencyResolutionDialog.tsx`: shown when enabling a mod triggers dependency chain

#### Edge Cases & Risks
- **Orphan references**: relationship targets a mod that was deleted. Show as "Missing: 'Base Mod'" with a red badge. Don't block enabling (user may have a renamed version, or the requirement is soft).
- **Transitive dependencies**: A requires B, B requires C. Enabling A must pull B and C. Show the full chain to user.
- **Dependency version constraints**: `version_min: "1.0"`. Requires semver comparison. If target mod has no version set, warn but don't block.
- **Conflicting requirements**: Mod A requires Mod X. Mod B conflicts with Mod X. User tries to enable both A and B. This is unresolvable — show clear error explaining the contradiction.
- **Self-referencing relationships**: Mod that requires or conflicts with itself. Validate and reject on import.
- **Load order chains**: A loads after B, B loads after C, C loads after A — circular. Detect cycle, report which mods form the cycle, refuse to auto-sort. Let user manually set priorities.

#### Hardening Analysis
- **Performance**: dependency graph is small (N mods, M relationships). BFS is O(N+M). Even 1000 mods with 5000 relationships is instant.
- **Manifest trust**: `skuld-manifest.json` comes from mod archives. Don't auto-execute anything from it — it's data only. Validate schema strictly. Reject manifests with invalid structure.
- **Migration**: existing mods have empty relationships. No behavioral change until user adds relationships.
- **UI complexity**: dependency resolution dialogs can be confusing. Show a tree diagram (ASCII art or simple indented list). "Enabling 'HD Rework' will also enable: Community Patch, Shared Assets. This will disable: Old Overhaul (conflict)." Clear, actionable.

#### Implementation Phases
1. Add `ModRelationEntry` and relationship types to config
2. Build `DependencyResult` resolver in `dependencies.rs`
3. Add manifest parsing to `archive.rs`
4. Build `DependencyResolutionDialog` frontend
5. Wire `toggle_mod` to call resolver before enabling
6. Build relationship editor in `ModInfoDialog`
7. Add relationship badges to `ModCard`

---

### 10. Download Manager

#### Summary
Download mods directly from Nexus Mods, GitHub, or direct URLs within the app. Show download progress, queue management, and auto-import after download completes.

#### Why
Current flow: browser download → find file in file manager → import dialog → pick file → import. For 20 mods, that's 100 steps. A download manager collapses this to: search → click download → mod appears in list.

#### Current State
- Mods imported from local archives only
- No HTTP download capability
- No progress tracking for any operations

#### Design

**Download sources:**

1. **Nexus Mods**: requires API key. Premium users get direct download links; free users get redirect to manual download page.
2. **GitHub Releases**: download release assets directly. No auth needed for public repos.
3. **Direct URL**: user pastes a URL. App downloads the file.

**Architecture:**

New module: `src-tauri/src/downloader.rs`

```rust
struct DownloadJob {
    id: String,
    game_id: String,
    mod_name: String,
    url: String,
    filename: String,
    total_bytes: Option<u64>,
    downloaded_bytes: u64,
    status: DownloadStatus,  // Queued, Downloading, Extracting, Completed, Failed
    error: Option<String>,
    created_at: String,
}
```

**Download flow:**

1. User opens "Download Mod" dialog (new)
2. Chooses source: Nexus, GitHub, or Direct URL
3. For Nexus: searches mods by game, picks a mod → picks a file → enters mod name → "Download & Import"
4. For GitHub: pastes repo URL → picks release asset → "Download & Import"
5. For Direct: pastes URL, enters mod name → "Download & Import"
6. Download begins in background. Progress shown in a new `DownloadQueue` panel.
7. On completion: auto-extracts to library, validates per game rules, adds to config as disabled.
8. Toast: "'Mod Name' downloaded and imported."

**Download queue:**

- Multiple downloads can run in parallel (configurable: 1-3 concurrent)
- Queue is visible in a slide-out panel or bottom bar
- Each item shows: mod name, progress bar (bytes + percentage), speed, ETA
- Cancel button per download
- "Clear completed" button

**Frontend:**

New components:
- `DownloadDialog.tsx`: source selection, search (for Nexus), file picker, mod name entry
- `DownloadQueue.tsx`: slide-out panel showing active + queued + completed downloads
- Download progress badges integrated into the app shell (persistent indicator: "2 downloading...")

New toolbar button in `ModList`: "Download Mod" (download icon)

**Nexus integration specifics:**

Nexus Mods API v2 (GraphQL):
- Search: `searchMods(gameId: 123, searchTerm: "HD Rework")`
- Mod details: `mod(uid: 1234) { name, summary, latestVersion, files { uid, name, version, size } }`
- Download: Premium users get direct CDN URL. Free users get a download page URL (open in browser).
- Rate limits: 2,500/day free, 25,000/day premium. Stagger requests.

**Backend commands:**

| Command | Purpose |
|---|---|
| `search_nexus_mods(game_type, query)` | Search Nexus for mods matching query |
| `get_nexus_mod_details(mod_uid)` | Get mod details + file list |
| `start_download(game_id, mod_name, url, filename)` | Begin a download job |
| `cancel_download(job_id)` | Cancel a running download |
| `get_download_status()` | Get status of all download jobs |
| `clear_completed_downloads()` | Remove completed/failed jobs from queue |

**Storage:**
- Downloaded archives saved to `~/.config/linux-mod-manager/downloads/` temporarily
- After extraction, archive is kept or deleted based on user preference (toggle: "Keep archives after import")
- Default: delete after successful import to save disk space

#### Integration Points
- `Cargo.toml`: add `reqwest` (already added for Feature 8), `tokio` for async downloads
- New file: `src-tauri/src/downloader.rs`
- `lib.rs`: add download commands, manage download queue state
- `archive.rs`: reuse extraction logic (call `import_mod`-style extraction from download completion)
- New `DownloadDialog.tsx`
- New `DownloadQueue.tsx`
- `ModList.tsx`: "Download Mod" button
- `App.tsx`: download queue state, new dialog handler

#### Edge Cases & Risks
- **Nexus premium vs free**: free users can't get direct download links. Mitigation: for free users, search works but "Download" opens the mod page in browser. Show clear messaging: "Nexus Premium required for in-app downloads. Opening browser instead."
- **Large files**: 5GB texture packs. Download to temp directory, show progress, handle disk full errors gracefully.
- **Network interruption**: resume support via HTTP Range header if the server supports it. If not, restart from beginning.
- **Simultaneous extraction + download**: extraction is CPU-heavy (7z). Don't extract while downloading — queue extraction after download completes. Or extract in separate thread.
- **7z dependency for extraction**: already handled by existing archive code. If 7z not installed, extraction fails — show clear error: "p7zip required to extract this mod."
- **Disk space**: check available space before download. Warn if free space < 2× file size.

#### Hardening Analysis
- **Download queue persistence**: queue should survive app restart. Serialize queue state to disk alongside config. On startup, resume incomplete downloads (if server supports Range).
- **Security**: downloaded files are arbitrary archives. Path traversal protection already exists in `archive.rs`. Add: scan for symlinks in archives (don't follow them), reject archives with absolute paths.
- **Temporary file cleanup**: if app crashes mid-download, partial files remain. Clean up partial downloads on startup (files older than 24h with no matching job).
- **Nexus API key security**: same as Feature 8. Store in config, mask in UI, validate on entry.
- **Modal during download**: user should be able to close the download dialog and keep browsing mods while downloads run. Downloads are background tasks, not modal-blocking operations.

#### Implementation Phases
1. Add `reqwest` + async HTTP to backend
2. Build download queue state management
3. Implement direct URL download with progress
4. Implement GitHub Releases download
5. Implement Nexus search + download (premium path first)
6. Build `DownloadDialog` frontend
7. Build `DownloadQueue` slide-out panel
8. Add auto-extract + import on download completion
9. Handle free-tier Nexus fallback (open in browser)

---

### 11. Mod Categories & Tags

#### Summary
Organize mods into categories (Gameplay, Graphics, UI, Audio, Patches, Overhaul, Utilities, Other) with support for custom user-defined tags. Filter and group mod list by category.

#### Why
Flat list of 100+ mods is hard to reason about. Categories let users filter: "show me only graphics mods" or "show me what patches I have." Tags add a second dimension for cross-cutting concerns: "performance-impact", "lore-friendly", "needs-new-game."

#### Current State
- No categorization exists
- `ModEntry` has no category or tag fields (will be added in Feature 4 as optional fields)

#### Design

This feature is mostly a **frontend refinement** on top of Feature 4's metadata. The category and tags fields already exist in the data model.

**Category system:**

Built-in categories (fixed list, extendable by user):
- `Gameplay` — mechanics, balance, new quests
- `Graphics` — textures, models, lighting, ENB/ReShade
- `UI` — menus, HUD, inventory, map
- `Audio` — music, sound effects, voice
- `Patches` — bug fixes, compatibility patches, translations
- `Overhaul` — total conversions, large-scale changes
- `Utilities` — tools, script extenders, loaders
- `Other` — catch-all

Users can add custom categories that apply per-game or globally.

**Tag system:**

Freeform tags. Suggested built-in tags (pre-defined for quick selection):
- `performance-impact` — mod affects framerate
- `lore-friendly` — fits game world
- `new-game-required` — needs fresh save
- `multiplayer-compatible` — works in co-op/multiplayer
- `controller-compatible`
- `high-priority` — should load early
- `low-priority` — can load anywhere

Users can create arbitrary custom tags.

**Frontend changes:**

`ModList` toolbar additions:
- Category filter: horizontal pill buttons or dropdown. Selecting a category filters the mod list. "All" is default. Shows count per category: "Graphics (12)", "Gameplay (8)".
- Tag filter: multi-select dropdown or inline tag chips. AND logic: show mods that have ALL selected tags.

`ModCard` additions:
- Category badge (colored, small) next to mod name
- Tag chips below metadata line (if any tags set)
- Clicking a tag on a mod card adds it to the active tag filter

`ModInfoDialog` additions:
- Category selector (dropdown with built-in + custom)
- Tag editor (text input + add button, chips with × to remove)
- Suggest existing tags as user types (autocomplete from all tags in the game)

**Sort options extended:**
Add "Category" as a sort option. Groups mods by category, then by name within each group.

#### Integration Points
- `ModEntry`: `category` and `tags` fields (from Feature 4)
- `ModList.tsx`: category filter pills, tag filter dropdown
- `ModCard.tsx`: category badge, tag chips, tag click handler
- `ModInfoDialog.tsx`: category selector, tag editor with autocomplete
- No backend changes (purely UI on existing data)

#### Edge Cases & Risks
- **Tag name collisions**: "high-priority" as both user tag and built-in tag. Treat as same tag. Merge in autocomplete.
- **Category filter + search interaction**: filter by category AND search text simultaneously. AND logic: mod must match both.
- **Empty category**: category filter shows "Patches (0)" — still show the pill, just greyed out. User can see it's empty.
- **Tag autocomplete performance**: collect all unique tags from game's mods. Even 500 mods × 5 tags = 2500 strings. Trivial.
- **Category persistence**: last-selected category filter should persist per game during session. Reset on app restart (or persist as user preference in config later).

#### Hardening Analysis
- **Dependency: requires Feature 4 (metadata)**: category and tags fields must exist.
- **Migration**: existing mods get category="Other" or None, tags=[].
- **Category vs tag confusion**: categories are mutually exclusive (one per mod). Tags are additive (many per mod). Document the distinction in the UI: "Category" label vs "Tags" label with different styling.
- **Filter combination matrix**: category + tags + search + sort. Four filter dimensions. Test all 16 combinations.

#### Implementation Phases
1. Define category enum and tag list (built-in sets)
2. Add category filter pills to `ModList` toolbar
3. Add tag filter dropdown to `ModList` toolbar
4. Add category badge to `ModCard`
5. Add tag chips to `ModCard`
6. Add category selector + tag editor to `ModInfoDialog`
7. Implement filter combination logic
8. Add tag autocomplete

---

## Tier 4 — Power User & Architecture

---

### 12. Plugin Management (Bethesda Games)

#### Summary
For Bethesda games (Skyrim, Fallout 4, Starfield), manage ESL/ESP/ESM plugin files with load order sorting (LOOT integration or built-in rules), dirty edit detection, and master dependency validation.

#### Why
Bethesda games are the largest modding ecosystem. Their mods use a plugin system (.esp/.esm/.esl files) with intricate load order rules. This is the core feature that makes Vortex and Mod Organizer 2 essential. Without plugin management, Skuld cannot credibly support Bethesda games.

#### Current State
- No plugin system support
- Load order for Witcher 3 is simple priority-based (just an integer)
- No concept of plugin files, masters, or records

#### Design

This is the most complex feature in the roadmap. It requires understanding of Bethesda's plugin architecture.

**Key concepts:**

- **ESM** (Elder Scrolls Master): master files. Always load first. Official DLC are ESMs.
- **ESP** (Elder Scrolls Plugin): standard plugin. Can have masters.
- **ESL** (Elder Scrolls Light): lightweight plugin. Doesn't count toward the 255 plugin limit. Can have masters.
- **ESL-flagged ESP**: ESP with ESL flag in header. Behaves like ESL.
- **Master dependency**: Plugin A has Plugin B as a master. Plugin B must load before Plugin A. If Plugin B is missing, game crashes on launch.
- **Plugin limit**: 255 total plugins (ESM + ESP). ESLs have their own limit of 4096.
- **Load order**: the sorted sequence of plugins. Determined by master dependencies + user overrides + LOOT rules.

**Plugin data model:**

```rust
struct PluginEntry {
    filename: String,           // e.g. "Skyrim.esm", "MyMod.esp"
    plugin_type: PluginType,    // ESM, ESP, ESL, ESLFlaggedESP
    masters: Vec<String>,       // plugins this depends on (e.g. ["Skyrim.esm", "Update.esm"])
    is_enabled: bool,
    load_order_index: u32,
    mod_id: Option<String>,     // links to ModEntry that provides this plugin
    record_count: Option<u32>,  // from plugin header
    is_dirty: bool,             // has ITMs or UDRs (detected by xEdit)
    dirty_info: Option<String>, // e.g. "3 ITM records, 2 deleted references"
    description: Option<String>, // from plugin header
}
```

**Plugin discovery:**

When a mod is imported for a Bethesda game, scan the extracted files for `.esm`, `.esp`, `.esl` files. Parse their headers (first ~200 bytes) to extract:
- Plugin type (byte 0x00: TES4 record header)
- Master list (MAST subrecords count and names)
- ESL flag (byte at offset determined by header flags)
- Record count
- Description (SNAM subrecord)

New Rust module: `src-tauri/src/plugins/bethesda.rs` implementing binary header parsing.

**Load order resolution:**

The app must produce a valid load order. Algorithm:

1. **Hard constraints (masters):** if A depends on B, B must load before A.
2. **LOOT integration:** call LOOT CLI (`loot-cli`) if installed. LOOT produces a sorted order based on its masterlist. If LOOT is not installed, use built-in rules.
3. **Built-in rules:** ship a small subset of LOOT rules for official DLC ordering. Most users will install LOOT.
4. **User overrides:** user can lock a plugin to a specific position or "load after" another plugin.
5. **Conflict resolution:** two plugins modify the same records. LOOT handles this; the app surfaces the conflict for user awareness.

**LOOT CLI integration:**

- Detect LOOT installation: check PATH for `loot`, check common install locations
- Call `loot --game="Skyrim Special Edition" --output=<path>` to get sorted order
- Parse LOOT output format
- If LOOT is not installed: show "Install LOOT for automatic load order sorting" message. Use basic master-based sort as fallback.

**Plugin deployment:**

Bethesda games load plugins from the game's `Data/` directory:
- Skyrim SE: `<game>/Data/`
- Fallout 4: `<game>/Data/`

Plugins can be deployed via symlink (like other mods) OR by enabling them in `plugins.txt`:
- Skyrim SE: `%LOCALAPPDATA%/Skyrim Special Edition/plugins.txt` (inside Proton prefix on Linux)
- Fallout 4: `%LOCALAPPDATA%/Fallout4/plugins.txt`

The `plugins.txt` file lists enabled plugins with `*` prefix:
```
# This file is used by Skyrim to keep track of your downloaded content.
Skyrim.esm
Update.esm
*MyMod.esp
*AnotherMod.esl
```

The app must:
1. Ensure plugins are in `Data/` (via symlink)
2. Write `plugins.txt` with correct load order
3. Handle the `sResourceArchiveList` and other INI settings if needed

**Archive virtualization (MO2-style):**

Mod Organizer 2 uses a virtual filesystem (USVFS) to present a merged view of mod files without touching the game directory. This is the gold standard but extremely complex to implement. For v1: use symlink-based deployment like existing games. This is what Vortex does by default. USVFS can be a future feature.

**Frontend:**

New view or section: "Plugins" tab in the main panel (alongside mod list).
- Plugin list with columns: checkbox (enabled), filename, type badge, load order index (editable), mod source, masters count, dirty flag
- "Sort with LOOT" button — runs LOOT, shows before/after diff, user confirms
- "Validate Masters" button — checks all plugins have their masters present and enabled
- Drag reorder (constrained by master dependencies — cannot drag above a master)
- Filter: show only plugins, only mods with plugins, dirty plugins, missing masters

#### Integration Points
- `Cargo.toml`: add `binread` or manual binary parsing for plugin headers
- New files: `src-tauri/src/plugins/mod.rs`, `src-tauri/src/plugins/bethesda.rs`
- `config.rs`: add `PluginEntry` struct, add `plugins` field to `GameEntry` (optional, only for Bethesda games)
- `archive.rs`: plugin discovery during import
- `lib.rs`: add commands: `scan_plugins`, `sort_plugins`, `validate_plugins`, `check_dirty_plugins`
- New components: `PluginList.tsx`, `PluginCard.tsx` (or extend `ModList` with tabbed view)
- `ModList.tsx`: add plugins tab

#### Edge Cases & Risks
- **Plugin limit**: 255 total ESP+ESM. ESL limit of 4096. App should show count and warn when approaching limits.
- **ESL flag manipulation**: some mods can have ESL flag added (if they have < 2048 records). This is an advanced operation. Don't auto-apply; offer as a tool with warnings.
- **LOOT not installed on Linux**: LOOT has a Linux build. Also available as a Flatpak. Detect both.
- **Proton paths**: plugins.txt is in the Proton prefix, not the game directory. Same path resolution logic as SoD2.
- **Creation Club content**: official micro-DLC. Have special naming and handling. Treat them as read-only mods. Don't let user disable them (game may require them for saves).
- **xEdit for dirty detection**: xEdit (`SSEEdit`, `FO4Edit`) runs on Windows. On Linux, it may run under Proton but calling it from the app is unreliable. Instead, ship a Rust-based minimal plugin header parser for dirty detection (detect ITMs by record hash comparison — complex). For v1: skip automatic dirty detection. Show a guide: "Use xEdit to check for dirty plugins."
- **Multiple profiles + plugins**: plugin load order is part of a profile (Feature 2). Switching profiles switches both mod states and plugin order.

#### Hardening Analysis
- **Master validation rigor**: missing master = guaranteed crash on launch. This check must be thorough. Check both presence in `Data/` AND enabled status in `plugins.txt`.
- **Load order cycles**: two plugins listing each other as masters is invalid but possible with corrupt files. Detect and reject.
- **Performance**: LOOT can take 5-30 seconds depending on load order size. Run it asynchronously, show progress.
- **User education**: Bethesda modding has a steep learning curve. Add in-app help tooltips and links to community guides. Don't assume user knows what an ESL is.

#### Implementation Phases
1. Build Bethesda plugin header parser in Rust
2. Implement plugin discovery during mod import
3. Build `scan_plugins` command (inventory all plugins in Data/)
4. Build `plugins.txt` reader/writer with load order support
5. Build master dependency validation
6. Integrate LOOT CLI for sorting
7. Build fallback sort (master-based) for when LOOT is unavailable
8. Build `PluginList` frontend component
9. Add plugins tab to `ModList` main panel
10. Wire profile integration (plugin order stored in profiles)

---

### 13. SQLite Backend

#### Summary
Replace the JSON config file with a SQLite database for mod, game, profile, plugin, and metadata storage. JSON remains as an export/import format.

#### Why
JSON config works for 2 games and 30 mods. At 10 games × 200 mods × 10 profiles × metadata × relationships × plugins, the JSON file becomes thousands of lines. Every toggle requires serializing and writing the entire config. SQLite gives: atomic updates, indexed queries, no full-file rewrite per operation, and proper data integrity.

#### Current State
- `AppConfig` is a single JSON struct, read and written as whole file
- `save_config` serializes all games + all mods + all profiles atomically
- No query capability — frontend receives entire config and filters in JS
- Config path: `~/.config/linux-mod-manager/config.json`

#### Design

**Schema design:**

```sql
-- Games
CREATE TABLE games (
    id TEXT PRIMARY KEY,
    game_type TEXT NOT NULL,
    name TEXT NOT NULL,
    install_path TEXT NOT NULL,
    launch_path TEXT,
    support_status TEXT NOT NULL DEFAULT 'verified',
    active_profile_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (active_profile_id) REFERENCES profiles(id)
);

-- Mods
CREATE TABLE mods (
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
    installed_at TEXT NOT NULL,
    updated_at TEXT,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
);

-- Installed files (one row per file, was a Vec in JSON)
CREATE TABLE installed_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mod_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE CASCADE
);

-- Tags (many-to-many)
CREATE TABLE tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE mod_tags (
    mod_id TEXT NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY (mod_id, tag_id),
    FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- Profiles
CREATE TABLE profiles (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
);

-- Profile mod states
CREATE TABLE profile_mod_states (
    profile_id TEXT NOT NULL,
    mod_id TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    priority INTEGER NOT NULL,
    PRIMARY KEY (profile_id, mod_id),
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
    FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE CASCADE
);

-- Relationships
CREATE TABLE mod_relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_mod_id TEXT NOT NULL,
    target_mod_id TEXT,
    target_mod_name TEXT,
    relation_type TEXT NOT NULL,  -- 'requires', 'conflicts', 'recommends', 'loads_after', 'loads_before', 'provides'
    note TEXT,
    FOREIGN KEY (source_mod_id) REFERENCES mods(id) ON DELETE CASCADE,
    FOREIGN KEY (target_mod_id) REFERENCES mods(id) ON DELETE SET NULL
);

-- Update sources
CREATE TABLE update_sources (
    mod_id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,  -- 'nexus', 'github', 'manual'
    nexus_game_domain TEXT,
    nexus_mod_id INTEGER,
    github_repo TEXT,
    manual_url TEXT,
    version_regex TEXT,
    FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE CASCADE
);

-- Backups (metadata, the actual backup files stay on disk)
CREATE TABLE backups (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    game_count INTEGER NOT NULL,
    mod_count INTEGER NOT NULL
);

-- Plugins (Bethesda)
CREATE TABLE plugins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    plugin_type TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    load_order_index INTEGER NOT NULL DEFAULT 0,
    mod_id TEXT,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE SET NULL
);

CREATE TABLE plugin_masters (
    plugin_id INTEGER NOT NULL,
    master_name TEXT NOT NULL,
    PRIMARY KEY (plugin_id, master_name),
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE
);

-- Schema version tracking
CREATE TABLE schema_version (
    version INTEGER NOT NULL
);
```

**Database location:** `~/.config/linux-mod-manager/skuld.db`

**Migration from JSON:**
1. On startup, check if `skuld.db` exists. If not, check for `config.json`.
2. If `config.json` exists and no DB: migrate. Read JSON, insert into SQLite tables, write `schema_version = 1`. Keep `config.json` as backup (rename to `config.json.v3.bak`).
3. On subsequent starts: use SQLite. `config.json` is only read if DB is missing/corrupt.

**Backend refactoring:**

Replace `config.rs` with `db.rs`:
- `get_config()` → `get_all_games()`, `get_game(id)`, `get_mods(game_id)`, etc.
- Frontend still receives typed structs (serde-serialized from DB rows). No raw SQL in frontend.
- All mutations use transactions: toggle mod = BEGIN, UPDATE mods SET enabled=..., UPDATE profile_mod_states..., COMMIT.
- `save_config()` becomes a no-op (auto-saved) or is removed.

**Dependency:** `rusqlite` with `bundled` feature (bundles SQLite, no system dependency).

#### Integration Points
- `Cargo.toml`: add `rusqlite` with `bundled` feature
- `config.rs` → refactored to `db.rs`
- All Tauri commands refactored to use DB instead of config
- `lib.rs`: DB initialization on startup, migration logic
- Frontend: mostly unchanged (still gets typed responses), but individual commands return partial data instead of full config

#### Edge Cases & Risks
- **Concurrent access**: Tauri runs a single backend instance. No concurrent DB access concerns.
- **Database corruption**: SQLite is robust but not immune to disk failures. Run `PRAGMA integrity_check` on startup. If corrupt, attempt repair. If repair fails, restore from latest backup (Feature 15 auto-backups).
- **Performance**: indexed queries. `SELECT * FROM mods WHERE game_id = ?` with index is instant even with 10,000 mods. No more full-config serialization.
- **Partial reads**: frontend can now request only what it needs. `get_game_mods(game_id)` returns just that game's mods, not all games. This fixes the scaling problem.
- **Transaction discipline**: every mutation must be transactional. Use a wrapper: `with_db(|conn| { conn.execute(...) })` that handles BEGIN/COMMIT/ROLLBACK.

#### Hardening Analysis
- **WAL mode**: enable WAL journal mode for better concurrency (even though single-instance, it's faster).
- **Foreign keys**: enable `PRAGMA foreign_keys = ON`. ON DELETE CASCADE ensures referential integrity.
- **Backup integration**: SQLite backup = `.dump` or VACUUM INTO. Offer both SQL dump (portable) and file copy (fast). Backup feature (Feature 15) stores timestamped copies of the DB file.
- **Testing**: all DB operations need integration tests. Use `rusqlite::Connection::open_in_memory()` for tests — no filesystem dependency.
- **Rollback strategy**: if migration from JSON fails, keep JSON config intact. Never delete it until DB is verified working.

#### Implementation Phases
1. Add `rusqlite` dependency
2. Design and create schema (migration 001)
3. Build `db.rs` module with typed query functions
4. Write JSON → SQLite migration
5. Refactor one command at a time (start with read-only: `get_config`)
6. Refactor mutation commands (start with `toggle_mod`)
7. Remove `config.rs` (keep structs, move to `models.rs`)
8. Add DB integrity check on startup
9. Add DB backup/restore
10. Full integration test suite

---

### 14. Cloud Sync

#### Summary
Sync the mod list, profiles, and metadata (not mod files) across machines via a user-provided cloud storage folder or a simple JSON export/import workflow.

#### Why
Users with desktop + Steam Deck want the same mod setup on both. Mod files may differ (different paths), but the list of enabled mods, load order, and profiles should sync. README says "No: cloud sync."

#### Current State
- Full config is local JSON (will be SQLite after Feature 13)
- Backup/restore is manual via dialog
- No sync mechanism

#### Design

This is NOT a cloud service. Skuld does not run servers. Two approaches, both offered:

**Approach A: Watch a synced folder (recommended)**

User points to a directory that is already synced by an external tool (Syncthing, Dropbox, Nextcloud, Steam Cloud, etc.):

1. User sets `sync_dir` in settings (e.g., `~/Sync/Skuld/`)
2. App exports a `sync-manifest.json` to that directory containing: mod list with enabled/priority states, profile data, metadata (not file paths — those are machine-specific)
3. On startup, app checks `sync_dir/sync-manifest.json` for changes
4. If remote manifest is newer, show "Sync changes detected" with a diff summary: "3 mods enabled, 1 disabled, new profile 'Steam Deck playthrough'"
5. User reviews and applies or rejects changes

**Sync manifest format:**

```json
{
  "schema_version": 1,
  "machine_id": "desktop-arch-2024",
  "last_synced": "2026-07-11T12:00:00Z",
  "games": [
    {
      "game_type": "witcher3",
      "machine_path_hint": "/mnt/games/Steam/steamapps/common/The Witcher 3",
      "active_profile_id": "abc123",
      "mods": [
        {
          "name": "HD Reworked Project",
          "version": "12.0",
          "enabled": true,
          "priority": 1,
          "source_url": "https://www.nexusmods.com/witcher3/mods/1021"
        }
      ],
      "profiles": [
        {
          "name": "Vanilla+",
          "mod_states": { "HD Reworked Project": { "enabled": true, "priority": 1 } }
        }
      ]
    }
  ]
}
```

**Conflict resolution:**
- If both machines changed the same mod: "Mod 'HD Reworked' changed on both machines. Desktop: enabled → disabled. Deck: enabled → enabled (no change)." User picks which to keep.
- If different mods changed on each machine: auto-merge (no conflict).
- Last-write-wins timestamp comparison for simple cases.

**Approach B: Manual export/import**

Simpler fallback:
- "Export Sync File" button → saves sync-manifest.json to user-chosen location (USB drive, network share, etc.)
- "Import Sync File" button → loads manifest, shows diff, user applies

This is essentially Backup/Restore but with machine-path independence. It reuses much of the existing BackupRestoreDialog.

**Frontend:**

New settings section or dialog: "Cloud Sync"
- Sync directory picker
- Last sync status: "Synced 2 minutes ago" or "Never synced"
- "Sync Now" button (force export now)
- Sync conflict resolution dialog (when importing with conflicts)
- Option: "Auto-apply sync on startup" (for hands-off Deck setups)

#### Integration Points
- `db.rs` or new `sync.rs`: export/import sync manifest
- `lib.rs`: add `export_sync_manifest`, `import_sync_manifest`, `check_sync_changes` commands
- Settings: add `sync_dir` and `auto_sync` to app settings (new or extend existing config)
- New `SyncDialog.tsx` or extend settings
- File watcher: `inotify` (Linux) to watch sync_dir for incoming changes. Not strictly necessary — polling on startup is simpler and sufficient for a game mod use case.

#### Edge Cases & Risks
- **Machine-specific paths**: mod install paths differ between machines. Manifest only stores mod names + states, not library paths. Each machine resolves its own paths.
- **Mod not installed on target machine**: manifest says "Mod X enabled" but Mod X doesn't exist on target. Show as "Missing: Mod X (not imported on this machine)" with a link to the source URL if available.
- **Game not configured on target machine**: manifest has a game not added to this machine. Skip it, show: "Sync includes Witcher 3, but it's not configured on this machine. Add it first."
- **Merge conflicts**: use CRDT-like approach or simple last-write-wins. CRDT is overkill — a three-way diff (base, ours, theirs) with user resolution for conflicts is sufficient.
- **Sync thrashing**: auto-export on every toggle would write constantly. Debounce exports: batch changes, export every 30 seconds or on app close.
- **Privacy**: sync manifest contains mod names and enabled states. No personal data. If stored in a cloud-synced folder, it's as private as the user's cloud storage.

#### Hardening Analysis
- **Simplicity over cleverness**: no custom sync protocol, no servers, no accounts. Just a JSON file in a user-managed folder. This is the Unix philosophy approach and avoids an entire category of bugs and security concerns.
- **Testing**: sync a manifest from machine A to machine B in test fixtures. Verify mod states match, paths are not hardcoded, missing mods are handled.
- **Documentation**: user needs to understand this is NOT automatic magic sync. They set up Syncthing/Dropbox/etc. themselves. Provide clear setup guides.

#### Implementation Phases
1. Define sync manifest format
2. Build `export_sync_manifest` command
3. Build `import_sync_manifest` command with conflict detection
4. Build sync settings UI (directory picker, auto-sync toggle)
5. Build sync conflict resolution dialog
6. Add startup sync check (if `sync_dir` is set)
7. Add debounced auto-export on config changes
8. Write user documentation for Syncthing/Dropbox setup

---

### 15. Script Extender Support

#### Summary
Detect, validate, and launch script extenders (SKSE, F4SE, REDmod, Cyber Engine Tweaks, etc.) for games that require them. Warn if a mod requires a script extender that isn't installed.

#### Why
Many mods depend on script extenders. If SKSE isn't installed, 90% of Skyrim mods won't work. The mod manager should detect this and warn users, ideally before they spend hours debugging why mods don't work.

#### Current State
- No script extender awareness
- Launch uses user-configured executable path only

#### Design

**Script extender registry:**

Define known script extenders per game:

```rust
struct ScriptExtender {
    name: String,                    // "Skyrim Script Extender"
    short_name: String,              // "SKSE"
    game_type: String,               // "skyrimse"
    executable_name: String,         // "skse64_loader.exe"
    required_files: Vec<String>,     // ["skse64_1_6_640.dll", "skse64_steam_loader.dll"]
    required_data_files: Vec<String>, // ["Data/Scripts/"]
    min_version_file: String,        // "skse64_loader.exe" — parse version from file
    steam_app_id: Option<u32>,       // if different from game's App ID
    website: String,                 // "https://skse.silverlock.org/"
    is_launcher: bool,               // true if you launch this instead of game exe
}
```

**Detection:**

On game add or on-demand scan:
1. Check game directory for known script extender executables
2. If found, parse version (SKSE embeds version string in the DLL/exe)
3. Compare with minimum version required by installed mods (from mod metadata)
4. Offer to set script extender as the launch executable (replacing game exe)

**Mod compatibility check:**

When a mod is imported, check if it has a manifest (`skuld-manifest.json` from Feature 9) that declares a script extender requirement. Or, scan mod files for script extender plugin patterns:
- `SKSE/Plugins/*.dll` → requires SKSE
- `F4SE/Plugins/*.dll` → requires F4SE
- `redmod/scripts/` → requires REDmod
- `bin/x64/plugins/cyber_engine_tweaks/` → requires CET

Auto-detect these patterns and set a `requires_extender` flag on the mod.

**Frontend:**

- Game panel header: script extender status badge. Green checkmark: "SKSE 2.2.6 detected." Red X: "SKSE not found. 12 mods require it."
- Mod card: "Requires SKSE" badge (amber) if mod has extender dependency
- Mod list safety note: extend the existing "close game before toggling mods" banner to include script extender warnings
- Launch button: if script extender is detected and is a launcher, use it automatically. Show "Launch (SKSE)" instead of "Launch."
- AddGameDialog: after adding a Bethesda game, automatically scan for script extenders and show results
- New `ScriptExtenderDialog`: detailed view of all extenders for a game, install instructions, download links

#### Integration Points
- New file: `src-tauri/src/script_extender.rs` — extender registry + detection logic
- Game definitions (Feature 6): add `script_extenders` field to JSON schema
- `archive.rs`: scan for extender plugin patterns during import
- `ModEntry`: add `requires_extender: Option<String>` field
- `lib.rs`: add `check_script_extenders(game_id)` command
- `ModList.tsx`: extender status badge in header
- `ModCard.tsx`: extender requirement badge
- `AddGameDialog.tsx`: post-add extender scan

#### Edge Cases & Risks
- **Multiple extender versions**: SKSE has AE (1.6.x) and SE (1.5.x) versions. User must install the one matching their game version. Detect game version from executable, suggest correct extender version.
- **Extender requires manual install**: SKSE has a specific install procedure (extract to game root, not just Data/). App cannot auto-install — point user to website, show instructions.
- **REDmod on GOG vs Steam**: different paths, different setup. GOG version may include REDmod by default.
- **Linux-specific extender issues**: SKSE works via Proton but may need specific launch options. Document this.
- **False positive detection**: mod with `SKSE/Plugins/` folder structure but for a different purpose. Flag as ambiguous — let user confirm or dismiss the requirement.

#### Hardening Analysis
- **Registry maintainability**: keep extender definitions in JSON alongside game definitions (Feature 6). Adding a new extender = adding a JSON block, not Rust code.
- **Detection reliability**: file existence checks are simple and reliable. Version parsing from binaries is fragile (string extraction). Fall back to "Version unknown" if parsing fails.
- **User confusion**: users new to modding may not know what SKSE is. Provide clear, friendly explanations. "SKSE is a tool that lets mods do more things. Without it, 12 of your mods won't work." Link to a beginner's guide.

#### Implementation Phases
1. Build script extender registry (data structure + JSON definitions)
2. Implement extender detection (executable + required files check)
3. Implement version detection (binary string extraction)
4. Implement mod extender dependency scanning on import
5. Add extender status to game panel header
6. Add extender requirement badges to mod cards
7. Build `ScriptExtenderDialog` with install guidance
8. Auto-set extender as launch executable when detected

---

## Feature Dependency Graph

```
Feature 4 (Metadata) ──────┬── Feature 8 (Update Checking)
                           ├── Feature 9 (Dependencies)
                           ├── Feature 11 (Categories & Tags)
                           └── Feature 15 (Script Extenders)

Feature 6 (Declarative Games) ──┬── Feature 5 (Auto-Detection) [mutual benefit]
                                └── Feature 7 (More Games)

Feature 2 (Profiles) ── independent (builds on existing config)
Feature 3 (Bulk Select) ── independent (builds on existing toggle/delete)
Feature 1 (Search/Filter) ── independent (pure frontend)

Feature 13 (SQLite) ── touches everything, best done after Tier 1+2 stable
Feature 12 (Bethesda Plugins) ── depends on Feature 7 (Bethesda games added)
Feature 10 (Download Manager) ── depends on Feature 4 (metadata for auto-import), Feature 8 (Nexus API integration)
Feature 14 (Cloud Sync) ── depends on Feature 13 (SQLite for efficient diffing) or can work with JSON
```

## Recommended Build Order

### Phase A: Foundation (Tier 1)
1. Feature 1: Search & Filter (1-2 days)
2. Feature 3: Bulk Select (1-2 days)
3. Feature 2: Profiles (3-4 days)

### Phase B: Data Model Expansion (Tier 2)
4. Feature 4: Metadata (2-3 days)
5. Feature 11: Categories & Tags (1 day, builds on Feature 4)
6. Feature 6: Declarative Game Definitions (3-4 days)
7. Feature 5: Game Auto-Detection (2-3 days)
8. Feature 7: Expanded Game Support (ongoing, first 4 games: 1-2 days each)

### Phase C: Smart Features (Tier 3)
9. Feature 9: Dependency Tracking (2-3 days)
10. Feature 8: Update Checking (2-3 days)
11. Feature 10: Download Manager (3-4 days)

### Phase D: Heavy Lifting (Tier 4)
12. Feature 13: SQLite Backend (3-4 days)
13. Feature 12: Plugin Management (Bethesda) (4-5 days)
14. Feature 15: Script Extender Support (1-2 days)
15. Feature 14: Cloud Sync (2-3 days)

### Phase E: Polish
- Accessibility audit (all features)
- Performance profiling (500+ mod scenarios)
- Error handling hardening (all network operations)
- User documentation
- Community game definition contribution guide

---

## Cross-Cutting Concerns

### Error Handling
- Every network operation (Features 5, 8, 10, 14): timeout, DNS failure, rate limit, HTTP error codes
- Every filesystem operation: permission denied, disk full, NTFS limitations, symlink failures
- Every user input: path traversal attempts, invalid JSON, SQL injection (mitigated by parameterized queries)

### Performance Targets
- App startup: < 2 seconds (with SQLite, 500 mods)
- Search/filter: < 16ms (60fps) for 500 mods
- Enable mod: < 500ms (symlink creation + conflict check)
- Dependency resolution: < 100ms for 500 mods with relationships
- Download: saturates available bandwidth
- LOOT sorting: < 30 seconds (external tool limitation)

### Accessibility
- All new UI: keyboard navigable, screen reader labels, focus management
- Color is never the sole indicator — all badges have text labels
- Focus rings on all interactive elements (already in design system)

### Testing Strategy Per Feature
- Feature 1-3 (frontend-heavy): React Testing Library + manual test with fixture data
- Feature 4, 6, 9, 13 (data model): Rust integration tests with test DB
- Feature 5, 8, 10 (external dependencies): mocked HTTP responses, real integration test with sandbox
- Feature 12 (plugins): binary fixture files (sample .esp headers)

### Migration Path for Existing Users
- Every config schema change (v3, v4, etc.) gets an automatic migration
- JSON → SQLite migration is one-time, with backup preservation
- Existing mods and games are never lost during migration
- Migration failures: keep original data intact, report error, let user fix manually
