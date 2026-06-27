//! The status-bar renderer.
//!
//! `render_bar` returns a single line (with ANSI truecolor escapes) that
//! zellij prints into the plugin's pane. The bar lists every tracked agent
//! as `❗ name · ⏳ name · ✅ name`, sorted by attention, plus a summary.

use crate::state::{PaneRecord, State};
use crate::status::Status;

/// Build the one-line status bar.
pub fn render_bar(state: &State, _rows: usize, cols: usize) -> String {
    if state.permissions_denied {
        return color(
            "eyegentic  permissions denied — grant ReadApplicationState, ReadPaneContents, ChangeApplicationState",
            (230, 70, 70),
        );
    }
    if !state.permissions_granted {
        return color("eyegentic  waiting for permissions…", (245, 166, 35));
    }

    let mut agents: Vec<&PaneRecord> = state.tracked.values().collect();
    agents.sort_by(|a, b| {
        b.status
            .attention_rank()
            .cmp(&a.status.attention_rank())
            .then_with(|| a.short_name.cmp(&b.short_name))
    });

    let mut out = String::new();
    out.push_str(&bold_color("eyegentic", (90, 200, 220)));
    out.push_str("  ");

    if agents.is_empty() {
        out.push_str(&color("no agents detected", (120, 120, 135)));
        return truncate(out, cols);
    }

    let mut parts: Vec<String> = Vec::new();
    for a in &agents {
        let (r, g, b) = a.status.color_rgb();
        parts.push(format!(
            "\x1b[38;2;{};{};{}m{} {}\x1b[0m",
            r,
            g,
            b,
            a.status.icon(),
            a.short_name
        ));
    }
    out.push_str(&parts.join(" \x1b[90m·\x1b[0m "));

    let needs_input = agents
        .iter()
        .filter(|a| a.status == Status::NeedsInput)
        .count();
    let errors = agents.iter().filter(|a| a.status == Status::Error).count();
    let working = agents
        .iter()
        .filter(|a| a.status == Status::Working)
        .count();

    let plural = if agents.len() == 1 { "" } else { "s" };
    let summary = format!(
        "{} agent{} · {} working · {} need input · {} error",
        agents.len(),
        plural,
        working,
        needs_input,
        errors
    );
    out.push_str("   ");
    out.push_str(&color(&summary, (120, 120, 135)));

    truncate(out, cols)
}

fn color(s: &str, (r, g, b): (u8, u8, u8)) -> String {
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, s)
}

fn bold_color(s: &str, (r, g, b): (u8, u8, u8)) -> String {
    format!("\x1b[1m\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, s)
}

/// Truncate a string to `cols` visible columns, accounting for the fact that
/// the string contains ANSI escapes (which don't consume columns).
fn truncate(s: String, cols: usize) -> String {
    if cols == 0 {
        return s;
    }
    let mut visible = 0usize;
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Copy the whole escape sequence through.
            out.push(c);
            if matches!(chars.peek(), Some('[')) {
                out.push(chars.next().unwrap());
                for d in chars.by_ref() {
                    out.push(d);
                    if ('@'..='~').contains(&d) {
                        break;
                    }
                }
            } else if let Some(d) = chars.next() {
                out.push(d);
            }
            continue;
        }
        if visible + 1 > cols {
            // Drop the rest; the bar is one line.
            break;
        }
        out.push(c);
        visible += 1;
    }
    out
}
