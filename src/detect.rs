//! Orchestration: walk the live pane manifest, decide which panes are agents,
//! classify each one's state, and rebuild [`State::tracked`].

use std::collections::BTreeMap;

use zellij_tile::prelude::*;

use crate::agent::{self, AgentDetector};
use crate::state::{PaneRecord, State};
use crate::status::Status;

/// Re-derive `state.tracked` from the current manifest + piped signals.
pub fn detect_all(state: &mut State) {
    let manifest = match state.pane_manifest.clone() {
        Some(m) => m,
        None => return,
    };
    let detectors = agent::detectors();
    let extra = &state.config.extra_agent_patterns;
    let scrollback_lines = state.config.scrollback_lines;

    let mut next: BTreeMap<u32, PaneRecord> = BTreeMap::new();
    let now = crate::state::unix_now();

    for (tab_position, panes) in &manifest.panes {
        for info in panes {
            // We only care about terminal panes (where agents live).
            if info.is_plugin {
                continue;
            }
            let id = info.id;
            let cmd = info.terminal_command.clone().unwrap_or_default();

            // 1) Is this pane an agent?
            let piped = state.piped.get(&id).copied();
            let mut is_agent = piped.is_some();
            if !is_agent {
                is_agent = detectors.iter().any(|d| d.matches_command(&cmd))
                    || matches_extra(&cmd, extra)
                    || agent::classify_title(&info.title).is_some();
            }
            if !is_agent {
                continue;
            }

            // 2) Classify — piped > title > scrollback > idle.
            let prev = state.tracked.get(&id);
            let mut status = piped.map(|p| p.status).unwrap_or(Status::Unknown);
            let mut via = if piped.is_some() { "pipe" } else { "infer" };
            // The tool name from the piped payload is stored on the record in
            // handle_pipe_payload; preserve it while a piped status is active.
            let tool = if piped.is_some() {
                prev.and_then(|p| p.tool.clone())
            } else {
                None
            };

            if status == Status::Unknown {
                if let Some(s) = best_classify(&detectors, &info.title, &cmd, scrollback_lines, id) {
                    status = s;
                    via = "infer";
                }
            }
            if status == Status::Unknown {
                // We know it's an agent but can't read a signal right now.
                status = Status::Idle;
            }

            // Stamp a change-time when the inferred status flips (for elapsed).
            let last_event_ts = match prev {
                Some(p) if p.status == status => p.last_event_ts,
                _ => {
                    let from = prev.map(|p| p.status).unwrap_or(crate::status::Status::Unknown);
                    crate::logln!(
                        "detect: pane {id} {:?} -> {:?} (via {via})",
                        from,
                        status
                    );
                    now
                }
            };
            let last_ts_ms = prev.map(|p| p.last_ts_ms).unwrap_or(0);

            let short_name = agent::display_name(&info.title, id);
            next.insert(
                id,
                PaneRecord {
                    pane_id: id,
                    tab_position: *tab_position,
                    title: info.title.clone(),
                    terminal_command: Some(cmd),
                    is_agent: true,
                    status,
                    short_name,
                    via,
                    tool,
                    last_event_ts,
                    last_ts_ms,
                },
            );
        }
    }

    let agents = next.len();
    state.tracked = next;
    if agents > 0 || state.tick % 10 == 0 {
        crate::logln!("detect: scan #{} tracked {agents} agent pane(s)", state.tick);
    }
}

/// Run each detector's classifier, backed by a scrollback read, returning the
/// first non-`None` status. Reads scrollback at most once per call.
fn best_classify(
    detectors: &[Box<dyn AgentDetector>],
    title: &str,
    _cmd: &str,
    scrollback_lines: usize,
    pane_id: u32,
) -> Option<Status> {
    // Try title-only first (cheap, no host call).
    for d in detectors {
        if let Some(s) = d.classify(title, &[]) {
            return Some(s);
        }
    }

    // Fall back to reading the pane's viewport.
    let viewport = match get_pane_scrollback(PaneId::Terminal(pane_id), false) {
        Ok(contents) => contents
            .viewport
            .into_iter()
            .map(|line| agent::strip_ansi(&line))
            .collect::<Vec<_>>(),
        Err(_) => return None,
    };

    for d in detectors {
        if let Some(s) = d.classify(title, &viewport) {
            return Some(s);
        }
    }
    let _ = scrollback_lines; // viewport already bounded by pane size
    None
}

fn matches_extra(command: &str, extra: &[String]) -> bool {
    if command.is_empty() || extra.is_empty() {
        return false;
    }
    let c = command.to_lowercase();
    extra.iter().any(|pat| c.contains(pat))
}
