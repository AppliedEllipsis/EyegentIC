//! Plugin configuration, parsed from the key/value map zellij hands to `load`.

use std::collections::BTreeMap;

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

        cfg
    }
}

fn parse_bool(v: &str) -> bool {
    matches!(
        v.trim().to_lowercase().as_str(),
        "true" | "1" | "yes" | "on" | "enabled"
    )
}
