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

    fn fingerprint(&self, viewport: &[String]) -> bool {
        pi_fingerprint_score(viewport) >= 2
    }

    fn classify(&self, title: &str, viewport: &[String]) -> Option<Status> {
        if let Some(s) = classify_title(title) {
            return Some(s);
        }
        classify_scrollback(viewport, 14)
    }
}

/// Score how strongly a viewport looks like pi's TUI. We require multiple
/// independent signals (see [`PiDetector::fingerprint`]) so a bare shell with
/// a starship `❯` prompt — which shares only the weakest single marker — can
/// never be mistaken for pi.
///
/// pi's footer renders two always-present lines regardless of agent state:
///   `<cwd> (<branch>)`
///   `↑6.3M ↓38k 12.7%/640k (auto)            (provider) <model> • <thinking>`
/// The token/context counter (`↑… ↓… N.N%/NNNk`) and the `model • thinking`
/// separator are highly pi-specific; the `❯` prompt and `Working…`/spinner are
/// supporting signals.
fn pi_fingerprint_score(viewport: &[String]) -> usize {
    if viewport.is_empty() {
        return 0;
    }
    // Inspect a generous tail — pi's footer sits at the bottom, but a tall
    // pane may push it up a few rows above a trailing blank line.
    let tail: Vec<&str> = viewport
        .iter()
        .rev()
        .take(24)
        .map(|s| s.as_str())
        .collect();
    let joined: String = tail.join("\n");
    let lower = joined.to_lowercase();

    let mut score = 0usize;

    // 1) Token-counter footer: an up-arrow and down-arrow on the same region
    //    plus a `NN%/` context-usage figure. Very pi-specific.
    let has_arrows = joined.contains('\u{2191}') && joined.contains('\u{2193}'); // ↑ ↓
    let has_ctx_pct = has_context_percent(&joined);
    if has_arrows && has_ctx_pct {
        score += 2; // strong enough on its own to pass the >= 2 gate
    } else if has_arrows || has_ctx_pct {
        score += 1;
    }

    // 2) The `(auto)` / `(sub)` cost indicator pi prints after the counters.
    if joined.contains("(auto)") || joined.contains("(sub)") {
        score += 1;
    }

    // 3) The model line's ` • ` separator between model name and thinking level.
    if joined.contains(" \u{2022} ") {
        score += 1;
    }

    // 4) Active-work markers: spinner glyph or pi's "working"/interrupt hints.
    if lower.contains("working\u{2026}")
        || lower.contains("working...")
        || lower.contains("to interrupt")
        || lower.contains("esc to interrupt")
        || tail.iter().any(|l| l.chars().any(|c| SPINNER.contains(c)))
    {
        score += 1;
    }

    // 5) The `❯` prompt glyph — weakest signal (starship uses it too), so it
    //    only ever contributes one point and never passes the gate alone.
    if joined.contains('\u{276f}') {
        score += 1;
    }

    score
}

/// Detect pi's context-usage figure: a `NN.N%/` or `NN%/` run (e.g.
/// `12.7%/640k`). We look for a `%` immediately followed by `/`, preceded by a
/// digit — distinctive enough to avoid matching prose percentages.
fn has_context_percent(s: &str) -> bool {
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'%' {
            let prev_digit = i > 0 && bytes[i - 1].is_ascii_digit();
            let next_slash = i + 1 < bytes.len() && bytes[i + 1] == b'/';
            if prev_digit && next_slash {
                return true;
            }
        }
    }
    false
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
