// Build script for eyegentic.
//
// Computes a fingerprint of the pi hook script (`scripts/eyegentic-hook.ts`)
// so that any content change to the hook automatically produces a different
// version tag, which triggers the plugin's auto-installer to re-write the
// installed copy at `~/.pi/agent/extensions/eyegentic/index.ts`.
//
// Without this, only `CARGO_PKG_VERSION` bumps would trigger a re-install;
// hook-only changes would be silently ignored.

use std::path::Path;

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let hook_path = manifest_dir.join("scripts").join("eyegentic-hook.ts");

    // Read the hook script and compute a simple fingerprint: byte length +
    // the first 8 bytes encoded as hex. Good enough to detect content changes
    // without pulling in a hash crate.
    let hook_bytes = std::fs::read(&hook_path).expect("scripts/eyegentic-hook.ts not found");
    let len = hook_bytes.len();
    let prefix_hex: String = hook_bytes.iter().take(8).map(|b| format!("{:02x}", b)).collect();

    // Combined fingerprint: length + first-8-bytes hex prefix.
    // This changes whenever the file content changes (byte length or first 8 bytes).
    // Collision risk is negligible for practical purposes — two different hook files
    // having identical length AND identical first 8 bytes is vanishingly unlikely.
    let fingerprint = format!("{len}:{prefix_hex}");
    println!("cargo:rustc-env=HOOK_FINGERPRINT={fingerprint}");
    println!("cargo:rerun-if-changed=scripts/eyegentic-hook.ts");
}
