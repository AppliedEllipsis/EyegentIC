//! The status-bar renderer.
//!
//! `render_bar` builds a single truecolor line that zellij prints into the
//! plugin's pane. It lists every tracked agent as `❗ name · ⏳ name · ✅ name`,
//! sorted by attention, with optional elapsed-time suffixes and a "needs
//! input" flash. It also records clickable regions so a click can focus a
//! waiting agent's pane or switch to its tab, plus a clickable settings menu.

use std::fmt::Write;

use zellij_tile::prelude::InputMode;

use crate::state::{MenuAction, State, ViewMode};
use crate::status::{tool_icon, Status};

type Color = (u8, u8, u8);

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";

const BRAND: Color = (90, 200, 220);
const SEP: Color = (96, 96, 120);
const SUM: Color = (120, 120, 135);
const FLASH_FG: Color = (255, 255, 80);
const FLASH_BG: Color = (80, 80, 30);
const WARN_FG: Color = (20, 20, 30);
const WARN_BG: Color = (255, 205, 0);

/// Build the one-line status bar (and populate `state.click_regions`).
pub fn render_bar(state: &mut State, _rows: usize, cols: usize) -> String {
    state.click_regions.clear();
    state.menu_click_regions.clear();
    state.prefix_click_region = None;

    if state.permissions_denied {
        return color(
            "eyegentic  permissions denied — grant ReadApplicationState, ReadPaneContents, ChangeApplicationState, RunCommands, ReadCliPipes, MessageAndLaunchOtherPlugins",
            (230, 70, 70),
        );
    }
    if !state.permissions_granted {
        return color("eyegentic  waiting for permissions…", (245, 166, 35));
    }

    let mut buf = String::with_capacity(cols * 6);
    // Keep the bar on one line, no cursor/wrap artifacts.
    let _ = write!(buf, "\x1b[H\x1b[?7l\x1b[?25l");

    // Prefix: " eyegentic (session) " + optional mode pill.
    let mut col = 0usize;
    let session = state
        .zellij_session_name
        .as_deref()
        .map(|n| format!(" ({n})"))
        .unwrap_or_default();
    let prefix_text = format!(" eyegentic{session} ");
    let _ = write!(buf, "{}{}{BOLD}{prefix_text}{RESET}", fg(BRAND), BOLD);
    col += char_width(&prefix_text);
    state.prefix_click_region = Some((0, col));

    // Mode pill (only in Normal view, to keep it unobtrusive).
    if state.view_mode == ViewMode::Normal {
        let (mc, mt) = mode_style(state.input_mode);
        let pill = format!(" {mt} ");
        let _ = write!(buf, "{}{}{BOLD}{pill}{RESET}", bg(mc), fg((20, 20, 30)));
        col += char_width(&pill);
    }

    if col + 2 <= cols {
        buf.push_str(&fg(SEP));
    }

    match state.view_mode {
        ViewMode::Normal => render_agents(state, &mut buf, &mut col, cols),
        ViewMode::Settings => render_settings_menu(state, &mut buf, &mut col, cols),
    }

    // Pad to the right edge with the default color so the bar fills the row.
    let visible = visible_width(&buf);
    if visible < cols {
        let _ = write!(buf, "{:width$}", "", width = cols - visible);
    }
    buf.push_str(RESET);
    buf
}

fn render_agents(state: &mut State, buf: &mut String, col: &mut usize, cols: usize) {
    let now = crate::state::unix_now();
    let threshold = state.config.elapsed_threshold;
    let show_elapsed = state.settings.elapsed_time;

    let mut agents: Vec<&crate::state::PaneRecord> = state.tracked.values().collect();
    agents.sort_by(|a, b| {
        b.status
            .attention_rank()
            .cmp(&a.status.attention_rank())
            .then_with(|| a.short_name.cmp(&b.short_name))
    });

    if agents.is_empty() {
        let _ = write!(buf, "  {}", color("no agents detected", SUM));
        *col += 2 + char_width("no agents detected");
        return;
    }

    // Prominent warning when one or more agents are blocked on you.
    let waiting: Vec<&str> = agents
        .iter()
        .filter(|a| a.status == Status::NeedsInput)
        .map(|a| a.short_name.as_str())
        .collect();
    if !waiting.is_empty() {
        let names = waiting.join(", ");
        let warn = if waiting.len() == 1 {
            format!(" \u{26a0} {names} is asking a question ")
        } else {
            format!(" \u{26a0} {} agents need input: {names} ", waiting.len())
        };
        let w = char_width(&warn);
        if *col + w + 2 <= cols {
            let _ = write!(buf, "  {}{}{BOLD}{warn}{RESET}", bg(WARN_BG), fg(WARN_FG));
            *col += w + 2;
        }
    }

    // Add spacing between the mode pill/prefix and the first agent.
    if *col + 2 <= cols {
        let _ = write!(buf, "  ");
        *col += 2;
    }

    let mut first = true;
    for a in &agents {
        let seg = render_segment(a, now, threshold, show_elapsed, state);
        let w = visible_width(&seg);
        if *col + w + (if first { 0 } else { 3 }) > cols {
            break;
        }
        if !first {
            let _ = write!(buf, " {}·{} ", fg(SEP), RESET);
            *col += 3;
        }
        let region_start = *col;
        buf.push_str(&seg);
        *col += w;
        state.click_regions.push(crate::state::ClickRegion {
            start_col: region_start,
            end_col: *col,
            tab_position: a.tab_position,
            pane_id: a.pane_id,
            is_agent: true,
            is_waiting: a.status == Status::NeedsInput,
        });
        first = false;
    }

    // Summary.
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
    let summary_w = char_width(&summary) + 3;
    if *col + summary_w <= cols {
        let _ = write!(buf, "   {}", color(&summary, SUM));
        *col += summary_w;
    }
}

fn render_segment(
    a: &crate::state::PaneRecord,
    now: u64,
    threshold: u64,
    show_elapsed: bool,
    state: &State,
) -> String {
    let flashing = state.is_flash_bright(a.pane_id);
    let icon = if a.status == Status::Working {
        a.tool.as_deref().map(tool_icon).unwrap_or(a.status.icon())
    } else {
        a.status.icon()
    };

    let mut s = String::new();
    if flashing {
        let _ = write!(s, "{}{}", bg(FLASH_BG), fg(FLASH_FG));
    } else {
        let (r, g, b) = a.status.color_rgb();
        let _ = write!(s, "{}", fg((r, g, b)));
    }
    let _ = write!(s, "{icon}  {}", a.short_name);

    // Elapsed-time suffix.
    if show_elapsed && a.status != Status::Idle && a.status != Status::Unknown {
        let elapsed = now.saturating_sub(a.last_event_ts);
        if elapsed >= threshold {
            let _ = write!(s, " {}", dim(&format_elapsed(elapsed)));
        }
    }
    if flashing {
        s.push_str(RESET);
    }
    s
}

fn render_settings_menu(state: &mut State, buf: &mut String, col: &mut usize, cols: usize) {
    let _ = write!(buf, "  ");
    *col += 2;

    add_toggle(state, buf, col, "bar", state.settings.status_bar, MenuAction::ToggleStatusBar);
    add_toggle(state, buf, col, "tabs", state.settings.rename_tabs, MenuAction::ToggleRenameTabs);
    add_toggle(state, buf, col, "panes", state.settings.rename_panes, MenuAction::ToggleRenamePanes);
    add_toggle(state, buf, col, "tint", state.settings.pane_tint, MenuAction::TogglePaneTint);
    add_toggle(
        state, buf, col, "elapsed", state.settings.elapsed_time, MenuAction::ToggleElapsedTime,
    );
    // Flash is a 3-state cycle.
    {
        let label = format!("flash:{}", state.settings.flash.label());
        let text = format!("● {label}");
        let start = *col;
        let _ = write!(buf, "{}{}  ", fg((255, 200, 60)), text);
        *col += char_width(&text) + 2;
        state.menu_click_regions.push(crate::state::MenuClickRegion {
            start_col: start,
            end_col: *col,
            action: MenuAction::CycleFlash,
        });
    }
    // Close button.
    {
        let start = *col;
        let _ = write!(buf, "{}×", fg((255, 60, 60)));
        *col += 1;
        state.menu_click_regions.push(crate::state::MenuClickRegion {
            start_col: start,
            end_col: *col,
            action: MenuAction::CloseMenu,
        });
    }
    let _ = cols;
}

fn add_toggle(
    state: &mut State,
    buf: &mut String,
    col: &mut usize,
    label: &str,
    on: bool,
    action: MenuAction,
) {
    let mark = if on { "●" } else { "○" };
    let mc = if on { (80, 200, 120) } else { (100, 100, 110) };
    let text = format!("{mark} {label}");
    let start = *col;
    let _ = write!(buf, "{}{}  ", fg(mc), text);
    *col += char_width(&text) + 2;
    state.menu_click_regions.push(crate::state::MenuClickRegion {
        start_col: start,
        end_col: *col,
        action,
    });
}

// ----- helpers -------------------------------------------------------------

fn mode_style(mode: InputMode) -> (Color, &'static str) {
    match mode {
        InputMode::Normal => ((80, 200, 120), "NORMAL"),
        InputMode::Locked => ((255, 80, 80), "LOCKED"),
        InputMode::Pane => ((80, 180, 255), "PANE"),
        InputMode::Tab => ((180, 140, 255), "TAB"),
        InputMode::Resize => ((255, 170, 50), "RESIZE"),
        InputMode::Move => ((255, 170, 50), "MOVE"),
        InputMode::Scroll => ((200, 200, 100), "SCROLL"),
        InputMode::EnterSearch => ((200, 200, 100), "SEARCH"),
        InputMode::Search => ((200, 200, 100), "SEARCH"),
        InputMode::RenameTab => ((200, 200, 100), "RENAME"),
        InputMode::RenamePane => ((200, 200, 100), "RENAME"),
        InputMode::Session => ((180, 140, 255), "SESSION"),
        InputMode::Prompt => ((80, 200, 120), "PROMPT"),
        InputMode::Tmux => ((80, 200, 120), "TMUX"),
    }
}

fn format_elapsed(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

fn fg((r, g, b): Color) -> String {
    format!("\x1b[38;2;{r};{g};{b}m")
}
fn bg((r, g, b): Color) -> String {
    format!("\x1b[48;2;{r};{g};{b}m")
}
fn color(s: &str, c: Color) -> String {
    format!("{}{s}{RESET}", fg(c))
}
fn dim(s: &str) -> String {
    format!("{DIM}{s}{RESET}")
}

fn char_width(s: &str) -> usize {
    s.chars().count()
}

/// Visible width of a string that may contain ANSI escapes.
fn visible_width(s: &str) -> usize {
    let mut n = 0usize;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if matches!(chars.peek(), Some('[')) {
                chars.next();
                for d in chars.by_ref() {
                    if ('@'..='~').contains(&d) {
                        break;
                    }
                }
            } else {
                chars.next();
            }
            continue;
        }
        if !c.is_control() {
            n += 1;
        }
    }
    n
}
