//! Auto-installs the pi extension that pipes agent state into eyegentic.
//!
//! On first load (after permissions are granted), the plugin writes a small
//! TypeScript extension to `~/.pi/agent/extensions/eyegentic/index.ts`. pi
//! auto-discovers extensions there — no manual registration is needed. The
//! install is idempotent: a version tag in the file lets us skip re-writing
//! when we're already current, and an older file is backed up (`.bak`) before
//! being overwritten.
//!
//! This is opt-out: set `auto_install_hook "false"` in the plugin config to
//! disable it. Removing the file is always safe — eyegentic falls back to
//! scrollback/title inference.

use zellij_tile::prelude::run_command;

// Version tag includes a content fingerprint so any change to the hook script
// automatically produces a different tag, triggering re-install even when the
// package version hasn't changed. The fingerprint is computed by build.rs.
const HOOK_VERSION_TAG: &str = concat!(
    "// eyegentic v",
    env!("CARGO_PKG_VERSION"),
    " [",
    env!("HOOK_FINGERPRINT"),
    "]",
);

fn hook_script_content() -> String {
    let original = include_str!("../scripts/eyegentic-hook.ts");
    format!("{HOOK_VERSION_TAG}\n{original}")
}

const INSTALL_TEMPLATE: &str = r##"set -e
EXT_DIR="$HOME/.pi/agent/extensions/eyegentic"
EXT_FILE="$EXT_DIR/index.ts"

# Already current? Skip.
if [ -f "$EXT_FILE" ] && grep -qF '__VERSION_TAG__' "$EXT_FILE" 2>/dev/null; then
  echo current
  exit 0
fi

mkdir -p "$EXT_DIR"

# Back up a previous, different version before overwriting.
if [ -f "$EXT_FILE" ]; then
  cp "$EXT_FILE" "$EXT_FILE.bak"
fi

cat > "$EXT_FILE" << 'EYEGENTIC_HOOK_EOF'
__HOOK_SCRIPT__
EYEGENTIC_HOOK_EOF

echo installed
"##;

/// Run the idempotent hook installation command.
pub fn run_install() {
    let cmd = INSTALL_TEMPLATE
        .replace("__VERSION_TAG__", HOOK_VERSION_TAG)
        .replace("__HOOK_SCRIPT__", &hook_script_content());

    let mut ctx = std::collections::BTreeMap::new();
    ctx.insert("type".into(), "install_hooks".into());
    run_command(&["sh", "-c", cmd.as_str()], ctx);
}
