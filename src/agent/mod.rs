//! Agent detection + state classification.
//!
//! This module is the extension point for supporting new coding agents. Each
//! agent implements [`AgentDetector`]; [`detectors`] returns the active set.
//! Detection combines three signals (per the plugin's design):
//!
//! 1. **command** — does the pane's running command look like this agent?
//! 2. **title** — does the pane's terminal title carry a status token?
//! 3. **scrollback** — does the pane's visible text match the agent's TUI?
//!
//! Piped messages (handled in [`crate::state::State::handle_pipe_payload`])
//! override inference and aren't part of this module.

pub mod pi;

use crate::status::Status;

/// One agent's recognition + classification rules.
pub trait AgentDetector: Send + Sync {
    /// Human-readable id, e.g. `"pi"`.
    fn name(&self) -> &'static str;

    /// Does this pane's running-command string look like this agent?
    fn matches_command(&self, command: &str) -> bool;

    /// Given the pane title and the (ANSI-stripped) viewport lines, classify
    /// the agent's state, or `None` if no signal is recognised.
    fn classify(&self, title: &str, viewport: &[String]) -> Option<Status>;
}

/// The active set of agent detectors. Add new agents here.
pub fn detectors() -> Vec<Box<dyn AgentDetector>> {
    vec![Box::new(pi::PiDetector)]
}

// ----- shared heuristics used by every detector ---------------------------

/// Braille spinner glyphs agents commonly use while "working".
const SPINNER: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏⠉⠑⠒⠓⠔⠕";

/// Classify from a terminal title alone.
///
/// pi's dynamic-title extension sets the title to a braille spinner while
/// running, a `●` when a task completes, and empty when idle. Other agents
/// sometimes embed words like "working" / "ready".
pub fn classify_title(title: &str) -> Option<Status> {
    let t = title.trim();
    if t.is_empty() {
        return None;
    }
    let first = t.chars().next().unwrap();
    if SPINNER.contains(first) {
        return Some(Status::Working);
    }
    let lower = t.to_lowercase();
    if lower.starts_with('●') {
        return Some(Status::Ready);
    }
    if lower.contains("error") || lower.contains("failed") || lower.contains("panic") {
        return Some(Status::Error);
    }
    if let Some(s) = Status::from_word(strip_status_word(&lower)) {
        return Some(s);
    }
    None
}

/// Pull the last `:`- or `|`-separated segment out of a title and see if it
/// is a status word. Returns the empty string if nothing useful is found.
fn strip_status_word(lower: &str) -> &str {
    for sep in ['|', ':', '—', '·'] {
        if let Some(idx) = lower.rfind(sep) {
            let tail = lower[idx + sep.len_utf8()..].trim();
            if !tail.is_empty() {
                return tail;
            }
        }
    }
    lower.trim()
}

/// Classify from the pane's visible viewport text (ANSI already stripped).
pub fn classify_scrollback(viewport: &[String], lines_to_inspect: usize) -> Option<Status> {
    // Work from the bottom up; agents put their status at the prompt line.
    let tail: Vec<&String> = viewport.iter().rev().take(lines_to_inspect).collect();
    if tail.is_empty() {
        return None;
    }

    let joined: String = tail
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let lower = joined.to_lowercase();

    // Highest-priority: an explicit question / options prompt.
    if lower.contains("do you want to proceed")
        || lower.contains("do you want to")
        || lower.contains("type something")
        || lower.contains("chat about this")
        || lower.contains("ask_user")
        || has_numbered_options(&lower)
    {
        return Some(Status::NeedsInput);
    }

    // Errors.
    if lower.contains("error:")
        || lower.contains("failed:")
        || lower.contains("panic:")
        || lower.contains("fatal:")
        || lower.contains("✘")
        || lower.contains("✗")
    {
        return Some(Status::Error);
    }

    // Working: spinner glyphs or activity words.
    if lower.chars().any(|c| SPINNER.contains(c))
        || lower.contains("working…")
        || lower.contains("working...")
        || lower.contains("thinking")
        || lower.contains("generating")
        || lower.contains("esc to interrupt")
    {
        return Some(Status::Working);
    }

    // Ready: an input prompt with no spinner and no question.
    let last_nonempty = tail
        .iter()
        .copied()
        .find(|s| !s.trim().is_empty())
        .unwrap_or(tail[0]);
    if last_nonempty.trim().contains('❯') {
        return Some(Status::Ready);
    }

    None
}

/// Detect a numbered-option menu like "1. Yes  2. No" near the prompt.
fn has_numbered_options(lower: &str) -> bool {
    let n = lower.matches("1. ").count() + lower.matches("1) ").count();
    n >= 1
        && (lower.contains("2. ") || lower.contains("2) ") || lower.contains("yes"))
}

// ----- ANSI stripping ------------------------------------------------------

/// Strip ANSI/CSI escape sequences so regex-free heuristics see plain text.
pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        // We saw ESC. Skip a CSI sequence: ESC [ ... <byte in 0x40..0x7e>
        if matches!(chars.peek(), Some('[')) {
            chars.next();
            for d in chars.by_ref() {
                if ('@'..='~').contains(&d) {
                    break;
                }
            }
            continue;
        }
        // ESC followed by a single char (e.g. OSC starts with ESC ]; we don't
        // bother parsing those fully — just drop the next char).
        chars.next();
    }
    out
}

/// A short, human-friendly name for a pane, derived from its title.
pub fn display_name(title: &str, pane_id: u32) -> String {
    let t = strip_our_icon_prefix(title).trim();
    let base = if t.is_empty() {
        format!("agent#{}", pane_id)
    } else {
        // Take the last path-like segment if it looks like a path.
        let seg = t.split([' ', '/']).last().unwrap_or(t).trim();
        if seg.is_empty() {
            t.to_string()
        } else {
            seg.to_string()
        }
    };
    // Truncate wide names so the status bar stays one line.
    if base.chars().count() > 18 {
        let mut s: String = base.chars().take(17).collect();
        s.push('…');
        s
    } else {
        base
    }
}

/// Strip a leading *our-icon* prefix (not an agent's braille spinner).
fn strip_our_icon_prefix(s: &str) -> &str {
    crate::status::strip_our_prefix(s)
}
