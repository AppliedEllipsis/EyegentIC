//! Diagnostic spike: dump the OS-level pane-query signals zellij exposes.
//!
//! This module exists to answer two questions on a real (esp. Windows) zellij
//! build, *before* we refactor detection around them:
//!
//!   1. Is the plugin actually past the permission gate? (If this logs at all,
//!      `ReadApplicationState` was granted and `tick()` is firing.)
//!   2. Which of zellij's per-pane OS queries actually return useful values
//!      here, and how do they compare to the cheap `PaneInfo` fields?
//!
//! For every terminal pane it logs, side by side:
//!   - `PaneInfo`: title, launch `terminal_command`, `exited`, `exit_status`
//!   - `get_pane_pid`        → the PID of the process in the pane
//!   - `get_pane_running_command` → the *current* foreground argv (sees through
//!     wrappers / npx / renamed binaries that the launch command hides)
//!   - `get_pane_cwd`        → the process's current working directory
//!
//! All three are `Result`, so an `Err(..)` line tells us the signal is
//! unavailable on this platform and we should fall back. Enable with
//! `probe "true"` in the layout; it is independent of `debug` for the file
//! sink but reuses the same log, so turn `debug "true"` on too to see output.
//!
//! This is a throwaway diagnostic — once we know which signals are real we
//! wire the useful ones into `detect.rs` and delete this module.

use zellij_tile::prelude::*;

use crate::state::State;

/// How often (in scans) to dump the probe. The OS queries are per-pane host
/// calls, so we don't run them every poll — every 5th scan is plenty to
/// eyeball the values without hammering the host.
const PROBE_EVERY: u64 = 5;

/// Walk the manifest and log the OS-query signals for each terminal pane.
pub fn probe_all(state: &State) {
    if !state.config.probe {
        return;
    }
    if state.tick % PROBE_EVERY != 0 {
        return;
    }
    let Some(manifest) = state.pane_manifest.as_ref() else {
        crate::logln!("probe: no pane_manifest yet (waiting on first PaneUpdate)");
        return;
    };

    crate::logln!("probe: ==== scan #{} ====", state.tick);
    for (tab_position, panes) in &manifest.panes {
        for info in panes {
            if info.is_plugin {
                continue;
            }
            let id = info.id;
            let pane_id = PaneId::Terminal(id);

            // Cheap PaneInfo fields (already in the manifest, no host call).
            crate::logln!(
                "probe: tab {tab_position} pane {id}: title={:?} launch_cmd={:?} exited={} exit_status={:?} focused={}",
                info.title,
                info.terminal_command,
                info.exited,
                info.exit_status,
                info.is_focused,
            );

            // 1) PID of the process running in the pane.
            match get_pane_pid(pane_id) {
                Ok(pid) => crate::logln!("probe:   pid -> {pid}"),
                Err(e) => crate::logln!("probe:   pid -> Err({e})"),
            }

            // 2) Current foreground argv (the strong identity signal). This is
            //    what should reveal a pi launched via wrapper / npx / rename.
            match get_pane_running_command(pane_id) {
                Ok(argv) => crate::logln!("probe:   running_cmd -> {argv:?}"),
                Err(e) => crate::logln!("probe:   running_cmd -> Err({e})"),
            }

            // 3) Current working directory of the pane's process.
            match get_pane_cwd(pane_id) {
                Ok(cwd) => crate::logln!("probe:   cwd -> {}", cwd.display()),
                Err(e) => crate::logln!("probe:   cwd -> Err({e})"),
            }
        }
    }
    crate::logln!("probe: ==== end scan #{} ====", state.tick);
}
