//! The pi coding agent detector.
//!
//! pi (`@earendil-works/pi-coding-agent`) is the first-class agent. It can be
//! launched as `pi` directly or via `node …/pi-coding-agent/…`. Its TUI uses
//! a `❯` prompt and — when paired with a title extension such as
//! `pi-dynamic-title` — a braille spinner in the terminal title.

use crate::agent::{classify_scrollback, classify_title, AgentDetector};
use crate::status::Status;

pub struct PiDetector;

impl AgentDetector for PiDetector {
    fn name(&self) -> &'static str {
        "pi"
    }

    fn matches_command(&self, command: &str) -> bool {
        let c = command.to_lowercase();
        if c.is_empty() {
            return false;
        }
        // Bare `pi` invocation or an npm-installed path.
        c == "pi"
            || c.contains("pi-coding-agent")
            || c.contains("/pi")
            || c.contains("\\pi")
            // `node …/pi` style launches.
            || (c.contains("node") && c.contains("pi"))
            // Common Windows path form.
            || c.contains("pi.exe")
    }

    fn classify(&self, title: &str, viewport: &[String]) -> Option<Status> {
        if let Some(s) = classify_title(title) {
            return Some(s);
        }
        classify_scrollback(viewport, 14)
    }
}
