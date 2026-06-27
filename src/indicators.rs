//! Applying indicators: tint panes, and prefix icons onto tab & pane names.
//!
//! All three are idempotent and guarded on "the desired indicator changed",
//! so we never fight the user or thrash zellij with redundant rename calls.

use zellij_tile::prelude::*;

use crate::state::State;
use crate::status::{strip_our_prefix, Status};

/// Re-apply every enabled indicator based on `state.tracked`.
pub fn apply(state: &mut State) {
    if !state.permissions_granted {
        return;
    }

    if state.config.pane_tint {
        tint_panes(state);
    }
    if state.config.rename_tabs {
        rename_tabs(state);
    }
    if state.config.rename_panes {
        rename_panes(state);
    }
}

fn tint_panes(state: &mut State) {
    let tracked: Vec<(u32, Status)> = state
        .tracked
        .iter()
        .map(|(id, r)| (*id, r.status))
        .collect();
    for (id, status) in tracked {
        let prev = state
            .pane_last_tint
            .get(&id)
            .copied()
            .unwrap_or(Status::Unknown);
        if prev == status {
            continue;
        }
        let (r, g, b) = status.color_rgb();
        let hex = format!("#{:02x}{:02x}{:02x}", r, g, b);
        set_pane_color(PaneId::Terminal(id), None, Some(hex));
        state.pane_last_tint.insert(id, status);
    }
}

fn rename_tabs(state: &mut State) {
    let tabs = match state.tabs.clone() {
        Some(t) => t,
        None => return,
    };
    for tab in &tabs {
        // Aggregate the agents living in this tab into one representative icon.
        let chosen = state
            .tracked
            .values()
            .filter(|r| r.tab_position == tab.position)
            .map(|r| r.status)
            .max_by_key(|s| s.attention_rank())
            .unwrap_or(Status::Unknown); // Unknown == "no agents / restore"

        let prev = state
            .tab_last_icon
            .get(&tab.tab_id)
            .copied()
            .unwrap_or(Status::Unknown);
        if prev == chosen {
            continue;
        }

        let original = state
            .tab_original
            .get(&tab.tab_id)
            .cloned()
            .unwrap_or_else(|| strip_our_prefix(&tab.name).to_string());
        let original = if original.is_empty() {
            "tab".to_string()
        } else {
            original
        };

        let desired = if chosen == Status::Unknown {
            original
        } else {
            format!("{} {}", chosen.icon(), original)
        };

        rename_tab(tab.position as u32, desired);
        state.tab_last_icon.insert(tab.tab_id, chosen);
    }
}

fn rename_panes(state: &mut State) {
    let tracked: Vec<(u32, String, Status)> = state
        .tracked
        .iter()
        .map(|(id, r)| (*id, r.title.clone(), r.status))
        .collect();
    for (id, title, status) in tracked {
        let prev = state
            .pane_last_icon
            .get(&id)
            .copied()
            .unwrap_or(Status::Unknown);
        if prev == status {
            continue;
        }
        let original = state
            .pane_original
            .get(&id)
            .cloned()
            .unwrap_or_else(|| strip_our_prefix(&title).to_string());
        let original = if original.is_empty() {
            format!("agent#{}", id)
        } else {
            original
        };
        let desired = format!("{} {}", status.icon(), original);
        rename_terminal_pane(id, desired);
        state.pane_last_icon.insert(id, status);
    }
}
