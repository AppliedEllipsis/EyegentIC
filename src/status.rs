//! The status taxonomy + visual mapping (icons, colors, ordering).
//!
//! Eyegent keeps an eye on every agent. Each tracked agent pane is classified into exactly one [`Status`], which
//! then drives the icon shown next to its name and the color it glows.

/// The finite set of states a coding-agent pane can be in.
///
/// Ordering is deliberate: variants are roughly ordered from "least needs
/// attention" to "most needs attention", which makes `max`-style aggregation
/// across the panes in a tab Just Work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    /// We have not been able to determine a state yet.
    #[default]
    Unknown,
    /// Agent is idle — alive but not actively working and not prompting.
    Idle,
    /// Agent finished and is ready for the next instruction.
    Ready,
    /// Agent is actively working / thinking / running a tool.
    Working,
    /// Agent encountered an error or failed.
    Error,
    /// Agent is blocked waiting for the human to answer a question / pick an option.
    NeedsInput,
}

impl Status {
    /// Unicode glyph shown next to the pane/tab name.
    pub fn icon(&self) -> &'static str {
        match self {
            Status::Unknown => "❔",
            Status::Idle => "⏸",
            Status::Ready => "✅",
            Status::Working => "⏳",
            Status::Error => "❌",
            Status::NeedsInput => "❗",
        }
    }

    /// Short human label, used in piped messages and debug output.
    pub fn label(&self) -> &'static str {
        match self {
            Status::Unknown => "unknown",
            Status::Idle => "idle",
            Status::Ready => "ready",
            Status::Working => "working",
            Status::Error => "error",
            Status::NeedsInput => "needs_input",
        }
    }

    /// Truecolor (R,G,B) used for the icon and (optionally) the pane tint.
    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            Status::Unknown => (150, 150, 160),
            Status::Idle => (120, 120, 135),
            Status::Ready => (80, 200, 120),
            Status::Working => (245, 166, 35),
            Status::Error => (230, 70, 70),
            Status::NeedsInput => (255, 205, 0),
        }
    }

    /// Higher = more deserving of the human's attention. Used to pick the
    /// representative icon for a tab that contains several agents.
    pub fn attention_rank(&self) -> u8 {
        match self {
            Status::Unknown => 0,
            Status::Idle => 1,
            Status::Ready => 2,
            Status::Working => 3,
            Status::Error => 4,
            Status::NeedsInput => 5,
        }
    }

    /// Parse a status word coming from a piped message or a title token.
    pub fn from_word(s: &str) -> Option<Status> {
        let s = s.trim().to_lowercase();
        if s.is_empty() {
            return None;
        }
        Some(match s.as_str() {
            "working" | "running" | "busy" | "thinking" | "generating" | "active" => {
                Status::Working
            }
            "ready" | "done" | "complete" | "completed" | "finished" | "ok" | "success" => {
                Status::Ready
            }
            "idle" | "waiting" | "standby" => Status::Idle,
            "needs_input" | "needs-input" | "input" | "question" | "asking" | "blocked"
            | "prompt" => Status::NeedsInput,
            "error" | "err" | "failed" | "failure" | "panic" | "crash" => Status::Error,
            _ => return None,
        })
    }
}

/// The set of icon glyphs eyegentic itself prefixes onto names, so we can tell
/// our own prefixes apart from an agent's title (e.g. pi's braille spinner).
pub const OUR_ICONS: &[&str] = &["❔", "⏸", "✅", "⏳", "❌", "❗"];

/// If `name` begins with one of our own icon prefixes (plus a space), return
/// the name with that prefix stripped. Otherwise return `name` unchanged.
pub fn strip_our_prefix(name: &str) -> &str {
    for icon in OUR_ICONS {
        let with_space = format!("{} ", icon);
        if let Some(rest) = name.strip_prefix(&with_space) {
            return rest;
        }
    }
    name
}

/// True if `name` looks like one we already prefixed.
pub fn has_our_prefix(name: &str) -> bool {
    strip_our_prefix(name) != name
}
