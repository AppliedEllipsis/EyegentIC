//! The pi coding agent detector.
//!
//! pi (`@earendil-works/pi-coding-agent`) is the first-class agent. It can be
//! launched as `pi` directly or via `node …/pi-coding-agent/…`. Its TUI uses
//! a `❯` prompt and — when paired with a title extension such as
//! `pi-dynamic-title` — a braille spinner in the terminal title.

use crate::agent::{classify_scrollback, classify_title, AgentDetector, SPINNER};
use crate::status::Status;

pub struct PiDetector;

/// pi's native app title glyph (U+03C0, "π"). pi sets the pane's terminal
/// title to `π - <session> - <cwd>` (or `π - <cwd>`) on every interactive
/// session — see pi's `config.js` `APP_TITLE`. This is present with no hook
/// and no title extension, so it's our most reliable detection signal for a
/// pi launched by typing `pi` at a shell prompt.
const PI_GLYPH: char = '\u{03c0}';

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

    fn matches_title(&self, title: &str) -> bool {
        // Strip any status icon we (or pi's spinner ext) prefixed, then look
        // for pi's native `π` app-title. We accept either the bare glyph at
        // the start (`π - cwd`) or a glyph following a leading spinner frame
        // (`⠋ π - cwd`, set by a title-spinner extension).
        let t = crate::status::strip_our_prefix(title).trim_start();
        let mut chars = t.chars();
        match chars.next() {
            Some(PI_GLYPH) => true,
            // Leading spinner frame, e.g. "⠋ π - cwd": skip the frame + space.
            Some(c) if SPINNER.contains(c) => {
                chars.as_str().trim_start().starts_with(PI_GLYPH)
            }
            _ => false,
        }
    }

    fn classify(&self, title: &str, viewport: &[String]) -> Option<Status> {
        if let Some(s) = classify_title(title) {
            return Some(s);
        }
        classify_scrollback(viewport, 14)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_native_pi_title() {
        let d = PiDetector;
        // pi's native title: `π - <cwd>` and `π - <session> - <cwd>`.
        assert!(d.matches_title("\u{03c0} - eyegentic"));
        assert!(d.matches_title("\u{03c0} - my-session - eyegentic"));
        // Leading/trailing whitespace tolerated.
        assert!(d.matches_title("  \u{03c0} - eyegentic"));
    }

    #[test]
    fn matches_pi_title_with_leading_spinner() {
        let d = PiDetector;
        // A title-spinner extension prepends a braille frame: `⠋ π - cwd`.
        assert!(d.matches_title("\u{280b} \u{03c0} - eyegentic"));
    }

    #[test]
    fn matches_pi_title_with_our_prefix() {
        let d = PiDetector;
        // After we prefix our own status icon, detection must still hold.
        assert!(d.matches_title("\u{23f3} \u{03c0} - eyegentic")); // ⏳ π - cwd
    }

    #[test]
    fn rejects_non_pi_titles() {
        let d = PiDetector;
        assert!(!d.matches_title(""));
        assert!(!d.matches_title("bash"));
        assert!(!d.matches_title("~/projects/foo"));
        assert!(!d.matches_title("vim src/main.rs"));
    }
}
