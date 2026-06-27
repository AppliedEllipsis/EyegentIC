//! The plugin's in-memory state and its lifecycle helpers.
//!
//! [`State`] is the single struct registered with zellij (via
//! `register_plugin!`). Everything else in the crate operates on a `&mut
//! State` handed to it from the `ZellijPlugin` trait impl in `lib.rs`.

use std::collections::BTreeMap;

use zellij_tile::prelude::*;

use crate::config::Config;
use crate::status::{has_our_prefix, strip_our_prefix, Status};

/// A status report pushed into the plugin from outside (e.g. a pi hook doing
/// `zellij pipe`). Piped signals take priority over inferred signals.
#[derive(Debug, Clone, Copy)]
pub struct PipedStatus {
    pub status: Status,
    pub tick: u64,
}

/// Everything eyegentic remembers about one tracked agent pane.
#[derive(Debug, Clone, Default)]
pub struct PaneRecord {
    pub pane_id: u32,
    pub tab_position: usize,
    pub title: String,
    pub terminal_command: Option<String>,
    pub is_agent: bool,
    pub status: Status,
    pub short_name: String,
    pub via: &'static str,
}

/// The plugin state. All fields are `Default`-able so the whole struct can be
/// constructed by zellij when it spawns the plugin.
#[derive(Default)]
pub struct State {
    pub config: Config,
    pub permissions_granted: bool,
    pub permissions_denied: bool,

    pub tabs: Option<Vec<TabInfo>>,
    pub pane_manifest: Option<PaneManifest>,

    /// pane_id -> record, for panes we believe are agents.
    pub tracked: BTreeMap<u32, PaneRecord>,
    /// tab_id -> the user's tab name (without our icon prefix).
    pub tab_original: BTreeMap<usize, String>,
    /// terminal pane_id -> the user's pane title (without our icon prefix).
    pub pane_original: BTreeMap<u32, String>,

    /// pane_id -> last status whose tint we applied (avoids re-tinting).
    pub pane_last_tint: BTreeMap<u32, Status>,
    /// tab_id -> last icon status we wrote into the tab name.
    pub tab_last_icon: BTreeMap<usize, Status>,
    /// terminal pane_id -> last icon status we wrote into the pane title.
    pub pane_last_icon: BTreeMap<u32, Status>,

    /// pane_id -> most recent piped status.
    pub piped: BTreeMap<u32, PipedStatus>,

    /// Monotonic tick counter (wraps at u64 max — practically never).
    pub tick: u64,
}

impl State {
    /// Called on every `Timer` fire: re-scan all panes, then re-apply indicators.
    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        if !self.permissions_granted {
            return;
        }
        crate::detect::detect_all(self);
        crate::indicators::apply(self);
    }

    /// Record the user's intended tab/pane names, stripping any prefix we added.
    /// Called whenever zellij sends us fresh `TabUpdate`/`PaneUpdate` state.
    pub fn sync_original_names(&mut self) {
        if let Some(tabs) = &self.tabs {
            for t in tabs {
                if has_our_prefix(&t.name) {
                    // This is a name we prefixed — keep the stored original.
                    self.tab_original
                        .entry(t.tab_id)
                        .or_insert_with(|| strip_our_prefix(&t.name).to_string());
                } else {
                    // The user (or zellij's auto-naming) set this name.
                    self.tab_original.insert(t.tab_id, t.name.clone());
                }
            }
        }
        if let Some(manifest) = &self.pane_manifest {
            for panes in manifest.panes.values() {
                for p in panes {
                    if p.is_plugin {
                        continue;
                    }
                    if has_our_prefix(&p.title) {
                        self.pane_original
                            .entry(p.id)
                            .or_insert_with(|| strip_our_prefix(&p.title).to_string());
                    } else {
                        self.pane_original.insert(p.id, p.title.clone());
                    }
                }
            }
        }
    }

    /// Handle a piped JSON payload (from `zellij pipe` or another plugin).
    ///
    /// Expected shape: `{"pane_id": <u32>, "status": "<word>", "agent": "pi"}`.
    pub fn handle_pipe_payload(&mut self, payload: &str) -> bool {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(payload);
        if let Ok(val) = parsed {
            let pane_id = val
                .get("pane_id")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);
            let status_word = val.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if let (Some(pid), Some(status)) = (pane_id, Status::from_word(status_word)) {
                self.piped.insert(
                    pid,
                    PipedStatus {
                        status,
                        tick: self.tick,
                    },
                );
                return true;
            }
        }
        false
    }
}
