# Skuld Mod Manager

A Tauri 2 desktop app for managing game mods on Linux. Import archives, toggle mods with symlinks, detect conflicts, and manage load order — all without touching a spreadsheet.

## Supported Games

| Game | Engine | Load Order | Mod Format | Status |
|------|--------|------------|------------|--------|
| The Witcher 3 | REDengine 3 | Yes | Folder with `content/` | Verified |
| State of Decay 2 | Unreal Engine 4 | No | `.pak` files | Provisional (Proton) |

State of Decay 2 mods deploy into the game's Proton prefix (`steamapps/compatdata/495420/pfx/.../AppData/Local/StateOfDecay2/Saved/Paks`), resolved automatically from the install path. Launch the game once through Steam before adding it, so the prefix exists.

## How It Works

1. **Add a game** — point to the install directory
2. **Import mods** — `.zip`, `.7z`, or `.rar` archives
3. **Toggle** — enable/disable mods with one click
4. **Deploy via symlinks** — no file copying, reversible
5. **Conflict detection** — blocks ambiguous overlaps (SoD2), warns where load order resolves them (Witcher 3)
6. **Drag reorder** — priority ordering for Witcher 3

## Build

### Prerequisites (Arch / CachyOS)

```bash
sudo pacman -S --needed webkit2gtk-4.1 base-devel openssl p7zip
```

### Dev

```bash
npm install
npm run tauri dev
```

### Release

```bash
cd src-tauri
cargo tauri build
```

Produces `.deb` and `.AppImage` in `src-tauri/target/release/bundle/`.

## Config

State lives at `~/.config/linux-mod-manager/config.json`. Mod files at `~/.config/linux-mod-manager/library/`. No database.

## Stack

- **Frontend:** React 18 + TypeScript + Vite
- **Backend:** Rust + Tauri 2
- **Design:** Custom dark theme (design tokens in source)
- **No:** database, cloud sync, Nexus API, auto-detection
