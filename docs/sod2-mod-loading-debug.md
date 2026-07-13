# SoD2 mod-loading debug — context for Claude on the gaming PC

## The problem
Skuld deploys enabled State of Decay 2 mods, but they don't take effect
in-game. No visible content/behaviour change. Reported on the gaming PC
(Linux + Steam Proton). Cannot be reproduced on the dev laptop — no SoD2
install there.

## What we already ruled out
The **deploy path is correct.** Skuld symlinks each mod's `.pak` into:

```
<steam-library>/steamapps/compatdata/495420/pfx/drive_c/users/steamuser/AppData/Local/StateOfDecay2/Saved/Paks/
```

Community + Nexus docs confirm that is the real load path for the Steam/Epic
build (`%LocalAppData%\StateOfDecay2\Saved\Paks`). Backend code:
`src-tauri/src/games/sod2.rs` (`resolve_proton_saved_dir` + `mod_dir`) and
`src-tauri/src/deploy.rs` (`create_symlink` / `deploy_mods_from_library`).

## Open suspects (why paks still don't load)
1. **Broken symlinks** — target file missing/moved, game sees nothing.
2. **Missing `_P` suffix** — UE4 only gives override priority to patch paks
   named `*_P.pak`. A plain `mod.pak` may not mount or override. Skuld keeps
   whatever filename the mod archive shipped, so this could be the mod, or a
   naming step Skuld should add.
3. **ModIntegrator step skipped** — some SoD2 mods require running
   `ModIntegrator.exe` → "Create Integration Pak" before they load. Skuld
   does no integration.
4. **Wrong prefix / rival AppData** — MS Store build reads a different
   `Packages\Microsoft.Dayton...\LocalCache\Local\...` path; if present the
   game may read there instead.
5. **Wine not following the Linux symlink** — unlikely (Wine reads Linux
   symlinks transparently) but unverified for this game.

## Do this
1. In Skuld, deploy at least one enabled SoD2 mod.
2. Run:
   ```bash
   bash scripts/diagnose-sod2.sh
   ```
3. Paste the full output into the chat, plus:
   - the exact mod name used, and whether its `.pak` filename ends in `_P`
   - whether the mod's Nexus page mentions "integration" / ModIntegrator

The script is read-only. It locates the prefix, lists deployed paks, checks
symlink validity + `_P` suffix, and looks for rival pak locations.
