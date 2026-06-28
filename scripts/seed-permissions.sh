#!/usr/bin/env bash
# Seed zellij's plugin-permission cache so eyegentic is granted on first load
# WITHOUT an interactive prompt.
#
# WHY THIS EXISTS
#   eyegentic is meant to live in a 1-row borderless status pane. zellij's
#   permission-approval prompt renders *inside the plugin's own pane* and needs
#   a multi-line y/n keypress — which a 1-row pane can't display or focus
#   (zellij bug zellij-org/zellij#4749). With the prompt unanswerable, zellij
#   never delivers Timer/PaneUpdate/etc. events, so the plugin loads and then
#   sits frozen forever: no status bar, and the pi hook never auto-installs.
#
#   zellij's server short-circuits the prompt entirely when the requested
#   permissions are already present in its on-disk cache. So we pre-seed that
#   cache. This is the documented workaround for the bug.
#
# WHAT IT WRITES
#   <zellij-cache-dir>/permissions.kdl   (created/merged, never clobbered)
#
#   The cache is keyed on the plugin's resolved location string
#   (RunPluginLocation::to_string()). For a `file:` plugin that's the resolved
#   path as Rust's PathBuf renders it. We can't know in advance exactly which
#   form will match (relative vs absolute, slash direction on Windows), and
#   lookup is an exact-match HashMap get, so we seed every plausible key. Extra
#   keys are harmless.
#
#   KDL GOTCHA: inside a quoted string, backslash is an escape char. A Windows
#   path written with single backslashes (e.g. "D:\_projects\...") contains
#   illegal escapes (\_, \z) that make the ENTIRE document fail to parse —
#   zellij then silently falls back to an empty cache, voiding every entry.
#   So every backslash in a key is doubled on disk.
#
# USAGE
#   scripts/seed-permissions.sh            # seed for this repo's eyegentic.wasm
#   WASM=/path/to/eyegentic.wasm scripts/seed-permissions.sh
#
# Safe to re-run: idempotent (skips keys already present).

set -euo pipefail
cd "$(dirname "$0")/.."
REPO_DIR="$(pwd)"

# --- locate the wasm ---------------------------------------------------------
WASM_PATH="${WASM:-$REPO_DIR/eyegentic.wasm}"
if [ ! -f "$WASM_PATH" ]; then
  echo "!! $WASM_PATH not found — run ./build.sh first (or set WASM=...)." >&2
  exit 1
fi

# --- the six permissions eyegentic requests in load() ------------------------
PERMS=(
  ReadApplicationState
  ChangeApplicationState
  RunCommands
  ReadCliPipes
  MessageAndLaunchOtherPlugins
  ReadPaneContents
)

# --- find zellij's cache dir (matches zellij_utils::consts) ------------------
#   Linux:   $XDG_CACHE_HOME/zellij      (default ~/.cache/zellij)
#   macOS:   ~/Library/Caches/org.Zellij-Contributors.Zellij
#   Windows: %LOCALAPPDATA%\Zellij\cache
detect_cache_dir() {
  local os
  os="$(uname -s 2>/dev/null || echo unknown)"
  case "$os" in
    Darwin)
      echo "$HOME/Library/Caches/org.Zellij-Contributors.Zellij"
      ;;
    Linux)
      echo "${XDG_CACHE_HOME:-$HOME/.cache}/zellij"
      ;;
    MINGW*|MSYS*|CYGWIN*)
      # Git-Bash/MSYS on Windows. LOCALAPPDATA is a Windows path; convert it.
      local la="${LOCALAPPDATA:-$HOME/AppData/Local}"
      if command -v cygpath >/dev/null 2>&1; then
        la="$(cygpath -u "$la")"
      fi
      echo "$la/Zellij/cache"
      ;;
    *)
      # Fallback: assume XDG-ish.
      echo "${XDG_CACHE_HOME:-$HOME/.cache}/zellij"
      ;;
  esac
}

CACHE_DIR="${ZELLIJ_CACHE_DIR_OVERRIDE:-$(detect_cache_dir)}"
CACHE_FILE="$CACHE_DIR/permissions.kdl"
mkdir -p "$CACHE_DIR"

# --- compute candidate keys --------------------------------------------------
# Absolute path, forward-slash form.
abspath_fwd="$WASM_PATH"
case "$abspath_fwd" in
  /*) : ;;                                  # already absolute (unix-ish)
  *)  abspath_fwd="$REPO_DIR/eyegentic.wasm" ;;
esac

declare -a KEYS=()
KEYS+=("eyegentic.wasm")                    # bare relative (what the layout uses)
KEYS+=("$abspath_fwd")                       # forward-slash absolute

# On Windows, zellij sees native backslash + drive-letter paths.
if command -v cygpath >/dev/null 2>&1; then
  win_abs="$(cygpath -w "$WASM_PATH" 2>/dev/null || true)"     # D:\_projects\...\eyegentic.wasm
  win_fwd="$(cygpath -m "$WASM_PATH" 2>/dev/null || true)"     # D:/_projects/.../eyegentic.wasm
  [ -n "$win_abs" ] && KEYS+=("$win_abs")
  [ -n "$win_fwd" ] && KEYS+=("$win_fwd")
fi

# --- emit one KDL block per key, backslashes doubled -------------------------
# Reads existing keys to stay idempotent.
existing=""
[ -f "$CACHE_FILE" ] && existing="$(cat "$CACHE_FILE")"

emit_block() {
  local key="$1"
  # Double every backslash for KDL string escaping.
  local esc="${key//\\/\\\\}"
  printf '"%s" {\n' "$esc"
  local p
  for p in "${PERMS[@]}"; do
    printf '    %s\n' "$p"
  done
  printf '}\n'
}

added=0
{
  # Preserve whatever's already there.
  [ -n "$existing" ] && printf '%s\n' "$existing"
  for key in "${KEYS[@]}"; do
    esc="${key//\\/\\\\}"
    # Skip if this exact escaped key line already exists.
    if printf '%s' "$existing" | grep -qF "\"$esc\" {"; then
      continue
    fi
    emit_block "$key"
    added=$((added + 1))
  done
} > "$CACHE_FILE.tmp"
mv -f "$CACHE_FILE.tmp" "$CACHE_FILE"

echo ">> cache file: $CACHE_FILE"
echo ">> seeded keys ($added new):"
for key in "${KEYS[@]}"; do echo "     $key"; done
echo ">> permissions: ${PERMS[*]}"
echo "   restart your zellij session (or reload the plugin) to pick up the grant."
