//! Plugin configuration, parsed from the key/value map zellij hands to `load`.
//!
//! These are *load-time* defaults. The toggleable subset is mirrored by the
//! live, persistable [`crate::settings::Settings`], which is seeded from this
//! config on first load and then driven by the in-bar settings menu.

use std::collections::BTreeMap;

use crate::settings::FlashMode;

/// All knobs the user can set through the plugin's layout configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Seconds between status scans (Timer-driven poll loop). Clamped to >= 0.2.
    pub poll_interval: f64,
    /// Draw the one-line status bar in the plugin pane.
    pub status_bar: bool,
    /// Tint each agent pane's default colors by state via `set_pane_color`.
    /// NOTE: zellij has no border-only color API, so this tints the pane's
    /// default background/foreground — experimental, defaults off.
    pub pane_tint: bool,
    /// Prefix tab names with the representative status icon of their agents.
    pub rename_tabs: bool,
    /// Prefix pane frame titles with the agent's status icon.
    pub rename_panes: bool,
    /// How many trailing viewport lines to inspect when parsing scrollback.
    pub scrollback_lines: usize,
    /// Extra substrings (case-insensitive) that mark a pane's command as an
    /// agent, beyond the built-in detector list. Comma-separated.
    pub extra_agent_patterns: Vec<String>,

    // --- load-time-only knobs (not toggled from the bar) -------------------

    /// Show "45s / 2m" next to an agent that has sat in the same state for at
    /// least this many seconds (helps spot stuck agents).
    pub elapsed_threshold: u64,
    /// Demote a *piped* Working status to Idle (letting inference take over)
    /// after this many seconds without a fresh piped event.
    pub working_stale_secs: u64,
    /// How the bar flashes when an agent starts needing input.
    pub flash: FlashMode,
    /// Show elapsed-time annotations in the bar.
    pub elapsed_time: bool,
    /// On first load (after permissions), auto-install a pi extension that
    /// pipes agent state into eyegentic. Idempotent + version-tagged.
    pub auto_install_hook: bool,
    /// Append timestamped debug lines to `/host/eyegentic.log` (the folder you
    /// ran `zellij -l` in). Watch with `tail -f eyegentic.log`.
    pub debug: bool,
    /// Diagnostic spike: dump per-pane OS-query signals (pid / running command
    /// / cwd) to the log so we can confirm which are available on this build
    /// before refactoring detection around them. Requires `debug "true"` for
    /// the log sink. Throwaway — see `src/probe.rs`.
    pub probe: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            poll_interval: 1.0,
            status_bar: true,
            pane_tint: false,
            rename_tabs: true,
            rename_panes: true,
            scrollback_lines: 14,
            extra_agent_patterns: Vec::new(),
            elapsed_threshold: 30,
            working_stale_secs: 600,
            flash: FlashMode::Brief,
            elapsed_time: true,
            auto_install_hook: true,
            debug: false,
            probe: false,
        }
    }
}

impl Config {
    /// Build a [`Config`] from the zellij-provided configuration map.
    pub fn from_configuration(c: &BTreeMap<String, String>) -> Self {
        let mut cfg = Self::default();

        if let Some(v) = c.get("poll_interval") {
            if let Ok(f) = v.parse::<f64>() {
                cfg.poll_interval = f.max(0.2);
            }
        }
        if let Some(v) = c.get("status_bar") {
            cfg.status_bar = parse_bool(v);
        }
        if let Some(v) = c.get("pane_tint") {
            cfg.pane_tint = parse_bool(v);
        }
        if let Some(v) = c.get("border_color") {
            // backwards-compatible alias; zellij has no border-only color API.
            cfg.pane_tint = parse_bool(v);
        }
        if let Some(v) = c.get("rename_tabs") {
            cfg.rename_tabs = parse_bool(v);
        }
        if let Some(v) = c.get("rename_panes") {
            cfg.rename_panes = parse_bool(v);
        }
        if let Some(v) = c.get("scrollback_lines") {
            if let Ok(n) = v.parse::<usize>() {
                cfg.scrollback_lines = n.max(1);
            }
        }
        if let Some(v) = c.get("extra_agent_patterns") {
            cfg.extra_agent_patterns = v
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
        }
        if let Some(v) = c.get("elapsed_threshold") {
            if let Ok(n) = v.parse::<u64>() {
                cfg.elapsed_threshold = n;
            }
        }
        if let Some(v) = c.get("working_stale_secs") {
            if let Ok(n) = v.parse::<u64>() {
                cfg.working_stale_secs = n;
            }
        }
        if let Some(v) = c.get("flash") {
            cfg.flash = match v.trim().to_lowercase().as_str() {
                "off" | "none" | "false" => FlashMode::Off,
                "persist" | "persistent" => FlashMode::Persist,
                _ => FlashMode::Brief,
            };
        }
        if let Some(v) = c.get("elapsed_time") {
            cfg.elapsed_time = parse_bool(v);
        }
        if let Some(v) = c.get("auto_install_hook") {
            cfg.auto_install_hook = parse_bool(v);
        }
        if let Some(v) = c.get("debug") {
            cfg.debug = parse_bool(v);
        }
        if let Some(v) = c.get("probe") {
            cfg.probe = parse_bool(v);
        }

        cfg
    }
}

fn parse_bool(v: &str) -> bool {
    matches!(
        v.trim().to_lowercase().as_str(),
        "true" | "1" | "yes" | "on" | "enabled"
    )
}
