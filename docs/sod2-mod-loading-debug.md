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

## Update from the gaming PC (2026-07-13)

Ran the same checks this script performs directly against the real install.
Found something upstream of all 5 suspects above: **the mod manager's DB had
zero `mods` rows for SoD2, even though 4 mods were fully extracted on disk**
in the library folder. `Saved/Paks` was empty — nothing was actually
deployed at the time of this check, so suspects #1–#3/#5 couldn't be tested
live yet.

Root cause: `remove_game` (`src-tauri/src/lib.rs`) cascade-deletes a game's
`mods` rows from SQLite but never touches the extracted library folder, and
nothing in the app ever looks at leftover library folders again afterward.
The old `RemoveGameDialog` copy said "mod library files stay on disk" —
true, but nothing let you get them back. Removing/re-adding the SoD2 entry
(plausible while troubleshooting the provisional Proton path detection)
silently orphaned all 4 mods.

Also ruled out **suspect #4 (rival AppData / MS Store path)**: only the
game's own `LivePatch-Windows-Data.pak` exists under the prefix's AppData,
no `Packages\Microsoft.Dayton...` path — this is a Steam build, not MS
Store, as expected. Base game install paks are present in the right
`Content/Paks` dir, confirming the correct Steam library/prefix.

One of the 4 orphaned folders ("All Bounties...") turned out to be a
pre-Paks-era mod — loose `.uasset` files under
`StateOfDecay2/Saved/Cooked/WindowsNoEditor/...`, no `.pak` at all. That one
can't be adopted or deployed under the current pak-based loader regardless
of the orphaning bug; treat it as an unrelated incompatible/legacy mod, not
part of this investigation.

**Fixed on `diagnostics/sod2-mod-loading`:**
- New `diagnose_game` command + "Diagnostics" toolbar button/dialog per
  game: reports install path, resolved mod dir, orphaned library folders,
  broken symlinks, and installed-file/library mismatches.
- New `adopt_orphaned_mod` command ("Recover" button in the Diagnostics
  dialog): re-validates an already-extracted library folder in place
  (no re-extraction) and registers it as a normal disabled mod.
- `RemoveGameDialog` copy now says mods become orphaned and points at
  Diagnostics → Recover, instead of implying an automatic safety net that
  didn't exist.

**Still open — needs testing on the gaming PC once mods are recovered:**
1. Open Skuld → select SoD2 → Diagnostics → Recover the 3 valid orphaned
   mods (all 3 already have `_P.pak` filenames, so suspect #2 is not the
   blocker for these specific mods — but keep it in mind for future imports
   that might not follow that convention).
2. Enable one, Deploy All, launch the game, check if it takes effect.
3. If it still doesn't load: suspects #3 (ModIntegrator step) and #5 (Wine
   symlink following) remain untested and are the next things to check.
