//! eyegentic — a zellij plugin that shows coding-agent pane status at a glance.
//!
//! eyegentic ("the eye, see" in agent-IC) keeps an eye on the coding-agent
//! panes in your zellij session and shows, at a glance, which one is
//! working, which is ready, and which is blocked waiting for you.
//!
//! Signals, in priority order:
//!   1. **piped** — a pi hook does `zellij pipe` with `{"pane_id","status"}`
//!   2. **title** — the pane's terminal title carries a status token
//!   3. **scrollback** — the pane's viewport matches the agent's TUI
//!
//! Indicators (all configurable, see [`config::Config`]):
//!   - a one-line status bar in the plugin pane
//!   - status icons prefixed onto tab names
//!   - status icons prefixed onto pane frame titles
//!   - (experimental) pane default-color tint by state
//!
//! See `README.md` for build + load instructions.

mod agent;
mod config;
mod detect;
mod indicators;
mod render;
mod state;
mod status;

use std::collections::BTreeMap;

use zellij_tile::prelude::*;

use crate::state::State;

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.config = config::Config::from_configuration(&configuration);

        // A status bar shouldn't steal focus.
        set_selectable(false);

        // Ask for everything we need up front; subscribe to the permission
        // result and to the timer (which needs no permission). We subscribe
        // to Tab/Pane updates only after the user grants permissions.
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ReadPaneContents,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[EventType::PermissionRequestResult, EventType::Timer]);

        // Kick off the polling loop. Timer fires even before permissions are
        // granted; `State::tick` no-ops until then.
        set_timeout(self.config.poll_interval);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                self.permissions_granted = true;
                subscribe(&[EventType::TabUpdate, EventType::PaneUpdate]);
                set_timeout(self.config.poll_interval);
                true
            }
            Event::PermissionRequestResult(PermissionStatus::Denied) => {
                self.permissions_denied = true;
                true
            }
            Event::TabUpdate(tabs) => {
                self.tabs = Some(tabs);
                self.sync_original_names();
                true
            }
            Event::PaneUpdate(manifest) => {
                self.pane_manifest = Some(manifest);
                self.sync_original_names();
                true
            }
            Event::Timer(_) => {
                self.tick();
                set_timeout(self.config.poll_interval);
                true
            }
            _ => false,
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if let Some(payload) = pipe_message.payload.as_deref() {
            return self.handle_pipe_payload(payload);
        }
        false
    }

    fn render(&mut self, rows: usize, cols: usize) {
        if self.config.status_bar {
            println!("{}", render::render_bar(self, rows, cols));
        }
    }
}
