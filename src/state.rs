//! The plugin's in-memory state and its lifecycle helpers.
//!
//! [`State`] is the single struct registered with zellij (via
//! `register_plugin!`). Everything else in the crate operates on a `&mut
//! State` handed to it from the `ZellijPlugin` trait impl in `lib.rs`.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use zellij_tile::prelude::*;

use crate::config::Config;
use crate::settings::Settings;
use crate::status::{has_our_prefix, strip_our_prefix, Status};

/// Wall-clock seconds since the Unix epoch. WASI preview1 exposes a realtime
/// clock, so `SystemTime::now()` works inside the plugin.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Two seconds of bright-yellow pulse on a "needs input" event.
pub const FLASH_DURATION_MS: u64 = 2000;

/// A status report pushed into the plugin from outside (e.g. a pi hook doing
/// `zellij pipe`). Piped signals take priority over inferred signals.
#[derive(Debug, Clone, Copy)]
pub struct PipedStatus {
    pub status: Status,
    /// Monotonic-ish send timestamp from the hook, used to drop out-of-order
    /// events that race through parallel hook invocations. `0` means
    /// "unspecified — treat as fresh".
    pub ts_ms: u64,
    /// When the piped status was received (unix seconds), for stale demotion.
    pub ts_secs: u64,
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
    /// Optional tool name from a richer piped payload — drives a per-tool icon.
    pub tool: Option<String>,
    /// Unix seconds of the last status change (for elapsed-time display).
    pub last_event_ts: u64,
    /// Highest hook `ts_ms` seen for this pane (out-of-order guard).
    pub last_ts_ms: u64,
}

/// Which in-bar view is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Normal,
    Settings,
}

/// A clickable region of the status bar. Clicking focuses the agent's pane if
/// it needs input, otherwise switches to its tab.
#[derive(Debug, Clone, Copy)]
pub struct ClickRegion {
    pub start_col: usize,
    pub end_col: usize,
    pub tab_position: usize,
    pub pane_id: u32,
    pub is_agent: bool,
    pub is_waiting: bool,
}

/// A serializable snapshot of a pane's state, used to sync across plugin
/// instances (one per tab). Only the fields needed to merge are carried.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncRecord {
    pub pane_id: u32,
    pub tab_position: usize,
    pub title: String,
    pub short_name: String,
    pub status: String,
    pub tool: Option<String>,
    pub last_event_ts: u64,
    pub last_ts_ms: u64,
    pub via: String,
}

/// The plugin state. All fields are `Default`-able so the whole struct can be
/// constructed by zellij when it spawns the plugin.
#[derive(Default)]
pub struct State {
    pub config: Config,
    pub settings: Settings,
    pub settings_loaded: bool,
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

    pub pane_last_tint: BTreeMap<u32, Status>,
    pub tab_last_icon: BTreeMap<usize, Status>,
    pub pane_last_icon: BTreeMap<u32, Status>,

    /// pane_id -> most recent piped status.
    pub piped: BTreeMap<u32, PipedStatus>,
    /// pane_id -> flash deadline (ms). `u64::MAX` means "persist".
    pub flash_deadlines: BTreeMap<u32, u64>,

    /// pane_id -> (tick when last fingerprinted, was-pi result). Lets us
    /// throttle the per-pane viewport read used for scrollback detection so we
    /// don't pay a host call for every untracked pane on every scan.
    pub fingerprint_cache: BTreeMap<u32, (u64, bool)>,

    pub input_mode: InputMode,
    pub zellij_session_name: Option<String>,
    pub view_mode: ViewMode,
    pub click_regions: Vec<ClickRegion>,
    pub prefix_click_region: Option<(usize, usize)>,
    pub menu_click_regions: Vec<MenuClickRegion>,

    pub hooks_installed: bool,
    pub tick: u64,
}

/// A clickable toggle in the settings menu.
#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    ToggleStatusBar,
    ToggleRenameTabs,
    ToggleRenamePanes,
    TogglePaneTint,
    ToggleElapsedTime,
    CycleFlash,
    CloseMenu,
}

#[derive(Debug, Clone, Copy)]
pub struct MenuClickRegion {
    pub start_col: usize,
    pub end_col: usize,
    pub action: MenuAction,
}

impl State {
    /// Called on every `Timer` fire: re-scan all panes, then re-apply indicators.
    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        if !self.permissions_granted {
            return;
        }
        crate::probe::probe_all(self);
        crate::detect::detect_all(self);
        self.cleanup_stale_piped();
        crate::indicators::apply(self);
    }

    /// Record the user's intended tab/pane names, stripping any prefix we added.
    pub fn sync_original_names(&mut self) {
        if let Some(tabs) = &self.tabs {
            for t in tabs {
                if has_our_prefix(&t.name) {
                    self.tab_original
                        .entry(t.tab_id)
                        .or_insert_with(|| strip_our_prefix(&t.name).to_string());
                } else {
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

    /// Handle a piped JSON payload from `zellij pipe` or another plugin.
    ///
    /// Accepted shapes:
    ///   status:   `{"pane_id":N,"status":"working"[,"tool":"bash"][,"ts_ms":N]}`
    ///   sync:      `{"sync":[SyncRecord,...]}`         (inter-plugin merge)
    ///   settings:  `{"settings":{...}}`                 (inter-plugin settings)
    pub fn handle_pipe_payload(&mut self, payload: &str) -> bool {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(payload);
        let val = match parsed {
            Ok(v) => v,
            Err(_) => return false,
        };

        // Inter-plugin sync: another instance sharing its tracked panes.
        if let Some(arr) = val.get("sync").and_then(|v| v.as_array()) {
            return self.merge_sync(arr);
        }
        // Inter-plugin settings broadcast.
        if let Some(s) = val.get("settings") {
            if let Ok(settings) = serde_json::from_value::<Settings>(s.clone()) {
                self.settings = settings;
                return true;
            }
        }

        // Hook / CLI status report.
        let pane_id = val.get("pane_id").and_then(|v| v.as_u64()).map(|n| n as u32);
        let status_word = val.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let tool = val
            .get("tool")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let ts_ms = val.get("ts_ms").and_then(|v| v.as_u64()).unwrap_or(0);

        let (Some(pid), Some(status)) = (pane_id, Status::from_word(status_word)) else {
            return false;
        };

        // Out-of-order guard: drop events older than the newest we've seen.
        if ts_ms > 0 {
            if let Some(prev) = self.piped.get(&pid) {
                if ts_ms < prev.ts_ms {
                    return false;
                }
            }
        }

        crate::logln!(
            "pipe: pane {pid} status={:?} tool={:?} ts_ms={ts_ms}",
            status,
            tool
        );

        self.piped.insert(
            pid,
            PipedStatus {
                status,
                ts_ms,
                ts_secs: unix_now(),
            },
        );

        // Update the tracked record's timestamp + tool so elapsed time & icons
        // reflect the piped event immediately (before the next detect pass).
        if let Some(rec) = self.tracked.get_mut(&pid) {
            rec.last_event_ts = unix_now();
            rec.last_ts_ms = ts_ms.max(rec.last_ts_ms);
            rec.status = status;
            rec.tool = tool.clone();
            rec.via = "pipe";
        }

        // Flash on needs-input, clear when it resolves away.
        if status == Status::NeedsInput {
            self.arm_flash(pid);
        } else {
            self.flash_deadlines.remove(&pid);
        }

        true
    }

    /// Demote a stale piped Working status to Idle so inference can take over
    /// again (e.g. an agent whose hook stopped reporting).
    fn cleanup_stale_piped(&mut self) {
        let now = unix_now();
        let stale_after = self.config.working_stale_secs;
        let to_clear: Vec<u32> = self
            .piped
            .iter()
            .filter_map(|(id, p)| {
                if p.status == Status::Working && now.saturating_sub(p.ts_secs) >= stale_after {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for id in to_clear {
            self.piped.remove(&id);
            if let Some(rec) = self.tracked.get_mut(&id) {
                rec.tool = None;
                rec.via = "infer";
            }
        }
    }

    pub fn arm_flash(&mut self, pane_id: u32) {
        if matches!(self.settings.flash, crate::settings::FlashMode::Off) {
            return;
        }
        let deadline = match self.settings.flash {
            crate::settings::FlashMode::Persist => u64::MAX,
            crate::settings::FlashMode::Brief | crate::settings::FlashMode::Off => {
                unix_now_ms() + FLASH_DURATION_MS
            }
        };
        self.flash_deadlines.insert(pane_id, deadline);
    }

    /// Drop expired brief flashes. Returns true if anything changed.
    pub fn cleanup_expired_flashes(&mut self) -> bool {
        let now = unix_now_ms();
        let before = self.flash_deadlines.len();
        self.flash_deadlines.retain(|_, d| *d == u64::MAX || now < *d);
        self.flash_deadlines.len() != before
    }

    pub fn has_active_flashes(&self) -> bool {
        let now = unix_now_ms();
        self.flash_deadlines.values().any(|&d| d == u64::MAX || now < d)
    }

    /// Is `pane_id` currently mid-flash (bright pulse tick)?
    pub fn is_flash_bright(&self, pane_id: u32) -> bool {
        self.flash_deadlines.get(&pane_id).map_or(false, |&d| {
            (d == u64::MAX || unix_now_ms() < d) && (unix_now_ms() / 250) % 2 == 0
        })
    }

    /// Merge a sync payload from another plugin instance. Newer `last_ts_ms`
    /// wins per pane; tab position is refreshed from our own manifest.
    fn merge_sync(&mut self, arr: &[serde_json::Value]) -> bool {
        let mut changed = false;
        for v in arr {
            let Ok(rec) = serde_json::from_value::<SyncRecord>(v.clone()) else {
                continue;
            };
            let dominated = self
                .tracked
                .get(&rec.pane_id)
                .map(|ex| rec.last_ts_ms > ex.last_ts_ms)
                .unwrap_or(true);
            if !dominated {
                continue;
            }
            let tool = rec.tool.clone();
            self.tracked.insert(
                rec.pane_id,
                PaneRecord {
                    pane_id: rec.pane_id,
                    tab_position: rec.tab_position,
                    title: rec.title.clone(),
                    terminal_command: None,
                    is_agent: true,
                    status: Status::from_word(&rec.status).unwrap_or(Status::Idle),
                    short_name: rec.short_name.clone(),
                    via: "sync",
                    tool,
                    last_event_ts: rec.last_event_ts,
                    last_ts_ms: rec.last_ts_ms,
                },
            );
            if let Some(s) = Status::from_word(&rec.status) {
                self.piped.insert(
                    rec.pane_id,
                    PipedStatus {
                        status: s,
                        ts_ms: rec.last_ts_ms,
                        ts_secs: unix_now(),
                    },
                );
            }
            changed = true;
        }
        changed
    }

    /// Serialize the current tracked set for a sync broadcast.
    pub fn sync_payload(&self) -> String {
        let recs: Vec<SyncRecord> = self
            .tracked
            .values()
            .map(|r| SyncRecord {
                pane_id: r.pane_id,
                tab_position: r.tab_position,
                title: r.title.clone(),
                short_name: r.short_name.clone(),
                status: r.status.label().to_string(),
                tool: r.tool.clone(),
                last_event_ts: r.last_event_ts,
                last_ts_ms: r.last_ts_ms,
                via: r.via.to_string(),
            })
            .collect();
        serde_json::to_string(&serde_json::json!({ "sync": recs })).unwrap_or_default()
    }
}
