#!/usr/bin/env bash
#
# diagnose-sod2.sh — figure out why State of Decay 2 mods deployed by Skuld
# don't load in-game (Linux / Proton).
#
# Read-only. Makes no changes. Run on the GAMING PC that has SoD2 installed,
# AFTER using Skuld to deploy at least one enabled mod.
#
#   bash scripts/diagnose-sod2.sh
#
# Copy the full output back into the Claude Code chat.
#
# Background / what we're testing:
#   Skuld symlinks each enabled mod's .pak into the game's Proton prefix at:
#     <steam-lib>/steamapps/compatdata/495420/pfx/drive_c/users/steamuser/
#       AppData/Local/StateOfDecay2/Saved/Paks/
#   Community + Nexus docs confirm that IS the correct load path for the
#   Steam/Epic build (%LocalAppData%\StateOfDecay2\Saved\Paks). So the path
#   is likely right; this script checks the *other* things that stop a pak
#   from loading:
#     1. Did the symlinks actually get created, and do they resolve to a
#        real file? (broken symlink = game sees nothing)
#     2. Do the pak filenames end in `_P.pak`? UE4 only gives override
#        priority to patch paks named `*_P.pak`. A plain `mod.pak` often
#        mounts too low to take effect, or not at all.
#     3. Is there a rival AppData location (MS Store build path) the game is
#        actually reading instead?
#     4. Does the base game pak exist where we expect (sanity: right prefix)?

set -uo pipefail
SOD2_APPID=495420

hr() { printf '\n========== %s ==========\n' "$1"; }

# --- 1. Find every Steam library root -----------------------------------
hr "STEAM LIBRARIES"
declare -a LIBS=()
for base in "$HOME/.steam/steam" "$HOME/.local/share/Steam" "$HOME/.var/app/com.valvesoftware.Steam/data/Steam"; do
  vdf="$base/steamapps/libraryfolders.vdf"
  if [[ -f "$vdf" ]]; then
    echo "found libraryfolders.vdf: $vdf"
    # pull every "path" value out of the vdf
    while IFS= read -r p; do
      [[ -n "$p" ]] && LIBS+=("$p")
    done < <(grep -oP '"path"\s*"\K[^"]+' "$vdf")
  fi
done
# always include the default roots themselves
LIBS+=("$HOME/.steam/steam" "$HOME/.local/share/Steam")
# de-dup
readarray -t LIBS < <(printf '%s\n' "${LIBS[@]}" | awk 'NF && !seen[$0]++')
printf '%s\n' "${LIBS[@]}"

# --- 2. Locate the SoD2 Proton prefix -----------------------------------
hr "PROTON PREFIX (compatdata/$SOD2_APPID)"
PFX=""
for lib in "${LIBS[@]}"; do
  cand="$lib/steamapps/compatdata/$SOD2_APPID/pfx"
  if [[ -d "$cand" ]]; then
    echo "FOUND prefix: $cand"
    PFX="$cand"
    break
  fi
done
if [[ -z "$PFX" ]]; then
  echo "NO prefix found in any library. Has SoD2 been launched once via Proton?"
  echo "Searching whole disk as fallback (may be slow)..."
  PFX=$(find "$HOME" -maxdepth 8 -type d -path "*compatdata/$SOD2_APPID/pfx" 2>/dev/null | head -1)
  echo "fallback result: ${PFX:-<none>}"
fi

# --- 3. Inspect the deploy target (Saved/Paks) --------------------------
hr "DEPLOY TARGET — Saved/Paks"
if [[ -n "$PFX" ]]; then
  PAKS="$PFX/drive_c/users/steamuser/AppData/Local/StateOfDecay2/Saved/Paks"
  echo "expected paks dir: $PAKS"
  if [[ -d "$PAKS" ]]; then
    echo "--- ls -la ---"
    ls -la "$PAKS"
    echo "--- symlink resolution + _P suffix check ---"
    shopt -s nullglob
    found_any=0
    for f in "$PAKS"/*; do
      found_any=1
      name=$(basename "$f")
      if [[ -L "$f" ]]; then
        tgt=$(readlink -f "$f" 2>/dev/null)
        if [[ -e "$tgt" ]]; then link="OK -> $tgt"; else link="BROKEN -> $(readlink "$f")"; fi
      else
        link="(regular file, not a symlink)"
      fi
      case "$name" in
        *_P.pak|*_p.pak) suf="has _P suffix (good for UE4 override)";;
        *.pak)           suf="!! NO _P suffix — may not mount/override";;
        *)               suf="not a .pak";;
      esac
      printf '  %-40s | %s | %s\n' "$name" "$suf" "$link"
    done
    [[ $found_any -eq 0 ]] && echo "  (empty — nothing deployed here)"
  else
    echo "!! Paks dir does not exist. Nothing deployed, or Saved/ not created yet."
    echo "   Saved/ contents:"
    ls -la "$PFX/drive_c/users/steamuser/AppData/Local/StateOfDecay2/Saved" 2>&1
  fi
fi

# --- 4. Rival AppData / MS Store location inside the prefix -------------
hr "RIVAL PAK LOCATIONS (any *.pak under the prefix's AppData)"
if [[ -n "$PFX" ]]; then
  find "$PFX/drive_c/users/steamuser/AppData" -iname '*.pak' 2>/dev/null || echo "(none)"
fi

# --- 5. Base game paks (sanity: correct install / prefix) ---------------
hr "BASE GAME INSTALL — Content/Paks"
for lib in "${LIBS[@]}"; do
  g="$lib/steamapps/common/StateOfDecay2"
  if [[ -d "$g" ]]; then
    echo "install: $g"
    find "$g" -iname '*.pak' 2>/dev/null | head -20
    echo "(base paks above — if empty, wrong install dir)"
    break
  fi
done

hr "DONE"
echo "Paste everything above back into the chat."
