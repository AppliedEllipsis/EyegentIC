//! Persisted, runtime-toggleable settings.
//!
//! [`Config`](crate::config::Config) holds the *load-time* defaults the user
//! set in their zellij layout. [`Settings`] holds the *live* values the status
//! bar actually reads — seeded from `Config` on first load, then mutated by the
//! in-bar settings menu and persisted to `~/.config/zellij/plugins/eyegentic.json`
//! so they survive restarts and sync across plugin instances.

use serde::{Deserialize, Serialize};

use crate::config::Config;

/// How aggressively to flash when an agent needs input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlashMode {
    /// Never flash.
    Off,
    /// Flash bright for a couple of seconds, then stop.
    Brief,
    /// Keep flashing until the need-input state clears.
    Persist,
}

impl Default for FlashMode {
    fn default() -> Self {
        FlashMode::Brief
    }
}

impl FlashMode {
    pub fn cycle(self) -> Self {
        match self {
            FlashMode::Off => FlashMode::Brief,
            FlashMode::Brief => FlashMode::Persist,
            FlashMode::Persist => FlashMode::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FlashMode::Off => "off",
            FlashMode::Brief => "brief",
            FlashMode::Persist => "persist",
        }
    }
}

/// The live, toggleable, persistable subset of configuration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub status_bar: bool,
    pub rename_tabs: bool,
    pub rename_panes: bool,
    pub pane_tint: bool,
    pub elapsed_time: bool,
    pub flash: FlashMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            status_bar: true,
            rename_tabs: true,
            rename_panes: true,
            pane_tint: false,
            elapsed_time: true,
            flash: FlashMode::Brief,
        }
    }
}

impl Settings {
    /// Seed live settings from the load-time config (used when no persisted
    /// settings file exists yet).
    pub fn from_config(c: &Config) -> Self {
        Self {
            status_bar: c.status_bar,
            rename_tabs: c.rename_tabs,
            rename_panes: c.rename_panes,
            pane_tint: c.pane_tint,
            elapsed_time: c.elapsed_time,
            flash: c.flash,
        }
    }
}
