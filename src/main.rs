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
//! Indicators (all configurable, see [`config::Config`] / [`settings::Settings`]):
//!   - a one-line status bar in the plugin pane (with elapsed time + flash)
//!   - status icons prefixed onto tab names
//!   - status icons prefixed onto pane frame titles
//!   - (experimental) pane default-color tint by state
//!
//! See `README.md` for build + load instructions.

mod agent;
mod config;
mod detect;
mod indicators;
mod installer;
#[macro_use]
mod log;
mod probe;
mod render;
mod settings;
mod state;
mod status;

use std::collections::BTreeMap;

use zellij_tile::prelude::*;

use crate::settings::Settings;
use crate::state::{MenuAction, State, ViewMode};

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.config = config::Config::from_configuration(&configuration);
        self.settings = Settings::from_config(&self.config);
        log::set_enabled(self.config.debug);
        logln!("load: debug logging on (poll={}s)", self.config.poll_interval);

        // NOTE: we deliberately do NOT call set_selectable(false) here. On
        // first run zellij shows its permission-approval prompt inside the
        // plugin's own pane and needs a keypress (`y`) to grant. If we hide
        // the pane up front it can never be focused to answer that prompt, so
        // permissions stay pending forever and tick() early-returns on every
        // timer. We pin the bar back to non-selectable in the Granted handler
        // once approval has come through.

        // Ask for everything up front. RunCommands + ReadCliPipes +
        // MessageAndLaunchOtherPlugins power the auto-installed hook, the
        // `zellij pipe` CLI bridge, and inter-instance sync respectively.
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ReadPaneContents,
            PermissionType::ChangeApplicationState,
            PermissionType::RunCommands,
            PermissionType::ReadCliPipes,
            PermissionType::MessageAndLaunchOtherPlugins,
        ]);
        subscribe(&[EventType::PermissionRequestResult, EventType::Timer]);

        set_timeout(self.config.poll_interval);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                self.permissions_granted = true;
                set_selectable(false);
                subscribe(&[
                    EventType::TabUpdate,
                    EventType::PaneUpdate,
                    EventType::ModeUpdate,
                    EventType::Mouse,
                    EventType::RunCommandResult,
                ]);
                self.load_settings();
                self.request_sync();
                if !self.hooks_installed && self.config.auto_install_hook {
                    installer::run_install();
                }
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
            Event::ModeUpdate(mode_info) => {
                self.input_mode = mode_info.mode;
                if let Some(name) = mode_info.session_name {
                    self.zellij_session_name = Some(name);
                }
                true
            }
            Event::Mouse(Mouse::LeftClick(line, col)) => {
                self.handle_click(line, col as usize);
                false
            }
            Event::RunCommandResult(exit_code, stdout, _stderr, context) => {
                match context.get("type").map(|s| s.as_str()) {
                    Some("load_settings") => {
                        if exit_code == Some(0) {
                            let raw = String::from_utf8_lossy(&stdout);
                            let trimmed = raw.trim();
                            if trimmed.is_empty() || trimmed == "{}" {
                                self.settings = Settings::from_config(&self.config);
                            } else if let Ok(s) = serde_json::from_str::<Settings>(trimmed) {
                                self.settings = s;
                            } else {
                                self.settings = Settings::from_config(&self.config);
                            }
                        }
                        self.settings_loaded = true;
                        true
                    }
                    Some("install_hooks") => {
                        self.hooks_installed = true;
                        false
                    }
                    Some("save_settings") => false,
                    _ => false,
                }
            }
            Event::Timer(_) => {
                // Surface the pending-permission stall in the log instead of
                // looking like a "blank"/frozen log. Logged sparsely so it
                // doesn't spam at the poll interval.
                if !self.permissions_granted
                    && !self.permissions_denied
                    && self.tick % 5 == 0
                {
                    crate::logln!(
                        "perm: still pending after {} ticks — approve the prompt in eyegentic's pane (press y); the bar pins itself back afterwards",
                        self.tick
                    );
                }
                self.tick();
                let _ = self.cleanup_expired_flashes();
                if self.has_active_flashes() {
                    set_timeout(0.25);
                } else {
                    set_timeout(self.config.poll_interval);
                }
                true
            }
            _ => false,
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        let name = pipe_message.name.as_str();
        let payload = pipe_message.payload.as_deref().unwrap_or("");
        match name {
            // Hook / CLI status report: {"pane_id":N,"status":"...","tool":...}
            "eyegentic" => self.handle_pipe_payload(payload),
            // Another instance asked for our state — broadcast it back.
            "eyegentic:request" => {
                self.broadcast_sync();
                false
            }
            // Another instance shared its state — merge.
            "eyegentic:sync" => self.handle_pipe_payload(payload),
            // Another instance broadcast settings — adopt them.
            "eyegentic:settings" => self.handle_pipe_payload(payload),
            // Notification click — focus the requested pane.
            "eyegentic:focus" => {
                if let Ok(pid) = payload.trim().parse::<u32>() {
                    focus_terminal_pane(pid, false, false);
                }
                false
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        if self.settings.status_bar {
            print!("{}", render::render_bar(self, rows, cols));
        }
    }
}

impl State {
    fn handle_click(&mut self, line: isize, col: usize) {
        // The bar is the plugin's single row. Only line 0 (or 1 in 1-based)
        // is the bar.
        if line != 0 && line != 1 {
            return;
        }

        // Prefix click toggles the settings menu.
        if let Some((start, end)) = self.prefix_click_region {
            if col >= start && col < end {
                self.view_mode = match self.view_mode {
                    ViewMode::Normal => ViewMode::Settings,
                    ViewMode::Settings => ViewMode::Normal,
                };
                return;
            }
        }

        match self.view_mode {
            ViewMode::Normal => {
                // Find the agent segment under the cursor.
                if let Some(region) = self
                    .click_regions
                    .iter()
                    .find(|r| col >= r.start_col && col < r.end_col)
                    .copied()
                {
                    if region.is_waiting {
                        focus_terminal_pane(region.pane_id, false, false);
                    } else {
                        switch_tab_to(region.tab_position as u32 + 1);
                    }
                }
            }
            ViewMode::Settings => {
                if let Some(region) = self
                    .menu_click_regions
                    .iter()
                    .find(|r| col >= r.start_col && col < r.end_col)
                    .copied()
                {
                    let action = region.action;
                    match action {
                        MenuAction::ToggleStatusBar => self.settings.status_bar ^= true,
                        MenuAction::ToggleRenameTabs => self.settings.rename_tabs ^= true,
                        MenuAction::ToggleRenamePanes => self.settings.rename_panes ^= true,
                        MenuAction::TogglePaneTint => self.settings.pane_tint ^= true,
                        MenuAction::ToggleElapsedTime => self.settings.elapsed_time ^= true,
                        MenuAction::CycleFlash => self.settings.flash = self.settings.flash.cycle(),
                        MenuAction::CloseMenu => self.view_mode = ViewMode::Normal,
                    }
                    // Closing the menu via × doesn't need a save; other toggles do.
                    if !matches!(action, MenuAction::CloseMenu) {
                        self.save_settings();
                    }
                }
            }
        }
    }

    // --- inter-plugin sync --------------------------------------------------

    fn request_sync(&self) {
        pipe_message_to_plugin(MessageToPlugin::new("eyegentic:request"));
    }

    fn broadcast_sync(&self) {
        let mut msg = MessageToPlugin::new("eyegentic:sync");
        msg.message_payload = Some(self.sync_payload());
        pipe_message_to_plugin(msg);
    }

    fn broadcast_settings(&self) {
        let mut msg = MessageToPlugin::new("eyegentic:settings");
        // Embed settings as a JSON object (Settings: Serialize + Copy) so the
        // receiver can serde_json::from_value it directly.
        msg.message_payload = Some(serde_json::json!({ "settings": self.settings }).to_string());
        pipe_message_to_plugin(msg);
    }

    // --- settings persistence ----------------------------------------------

    fn load_settings(&self) {
        let mut ctx = BTreeMap::new();
        ctx.insert("type".into(), "load_settings".into());
        run_command(
            &[
                "sh",
                "-c",
                "cat \"$HOME/.config/zellij/plugins/eyegentic.json\" 2>/dev/null || echo '{}'",
            ],
            ctx,
        );
    }

    fn save_settings(&self) {
        if !self.settings_loaded {
            return;
        }
        self.broadcast_settings();
        let Ok(json) = serde_json::to_string(&self.settings) else {
            return;
        };
        let json_esc = json.replace('\'', "'\\''");
        let cmd = format!(
            "mkdir -p \"$HOME/.config/zellij/plugins\" && printf '%s' '{json_esc}' > \"$HOME/.config/zellij/plugins/eyegentic.json\""
        );
        let mut ctx = BTreeMap::new();
        ctx.insert("type".into(), "save_settings".into());
        run_command(&["sh", "-c", cmd.as_str()], ctx);
    }
}
