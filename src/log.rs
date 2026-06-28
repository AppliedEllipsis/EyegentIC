//! Lightweight debug logging to a file you can `tail -f`.
//!
//! zellij runs the plugin in a WASI sandbox whose only easily-discoverable
//! writable mount is `/host` — the directory `zellij` was launched from. So
//! when `debug` is on we append newline-delimited, timestamped lines to
//! `/host/eyegentic.log`. Watch it live with:
//!
//! ```sh
//! tail -f eyegentic.log     # from the folder you ran `zellij -l` in
//! ```
//!
//! Logging is a no-op unless [`set_enabled`] was called with `true` (driven by
//! the `debug "true"` layout knob). We never panic on IO errors — a logger
//! that takes down the plugin is worse than no logger.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::state::unix_now_ms;

static ENABLED: AtomicBool = AtomicBool::new(false);

const LOG_PATH: &str = "/host/eyegentic.log";

/// Turn file logging on or off (called from `load` based on config).
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Append one line to the log file. Silently does nothing when disabled or if
/// the file can't be opened (read-only `/host`, etc.).
pub fn line(msg: &str) {
    if !enabled() {
        return;
    }
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)
    {
        // ms-since-epoch is enough to correlate events; no date formatting dep.
        let _ = writeln!(f, "[{}] {}", unix_now_ms(), msg);
    }
}

/// `log::line(format!(...))` ergonomics without allocating when disabled.
#[macro_export]
macro_rules! logln {
    ($($arg:tt)*) => {{
        if $crate::log::enabled() {
            $crate::log::line(&format!($($arg)*));
        }
    }};
}
