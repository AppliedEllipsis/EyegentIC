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
            // Diagnostic: dump what we actually see for EVERY pane (incl.
            // plugins), so we can tell whether pi's title/command reach us and
            // how panes are flagged. Sparse (every 10 scans) to avoid spamming.
            if state.tick % 10 == 0 {
                crate::logln!(
                    "  pane {}: title={:?} cmd={:?} is_plugin={}",
                    info.id,
                    info.title,
                    info.terminal_command,
                    info.is_plugin
                );
            }

            // We only care about terminal panes (where agents live).
            if info.is_plugin {
                continue;
            }
            let id = info.id;
            let cmd = info.terminal_command.clone().unwrap_or_default();

            // 1) Is this pane an agent? Cheap signals first (no host call):
            //    piped status, running command, native title, extra patterns.
            let piped = state.piped.get(&id).copied();
            let mut is_agent = piped.is_some()
                || detectors.iter().any(|d| d.matches_command(&cmd))
                || detectors.iter().any(|d| d.matches_title(&info.title))
                || matches_extra_any(&cmd, &info.title, extra)
                || agent::classify_title(&info.title).is_some();

            // 2) Read the viewport once (host call) — needed for scrollback
            //    *detection* (fingerprint) and/or *classification*. We only
            //    pay for it when a pane is either already an agent (to classify
            //    it) or still a detection candidate we should fingerprint.
            //    Fingerprinting is throttled per-pane via the cache so an
            //    untracked shell isn't re-read on every single scan.
            let mut viewport: Option<Vec<String>> = None;
            let mut fp_via_scrollback = false;
            if !is_agent {
                let due = match state.fingerprint_cache.get(&id) {
                    Some((last, was_pi)) => {
                        // Re-check sooner if it looked like pi last time.
                        let gap = if *was_pi { FP_GAP_HIT } else { FP_GAP_MISS };
                        state.tick.wrapping_sub(*last) >= gap
                    }
                    None => true,
                };
                if due {
                    let vp = read_viewport(id);
                    let is_pi = !vp.is_empty()
                        && detectors.iter().any(|d| d.fingerprint(&vp));
                    state.fingerprint_cache.insert(id, (state.tick, is_pi));
                    if is_pi {
                        is_agent = true;
                        fp_via_scrollback = true;
                    }
                    viewport = Some(vp);
                } else if matches!(state.fingerprint_cache.get(&id), Some((_, true))) {
                    // Cached as pi within the throttle window — trust it.
                    is_agent = true;
                    fp_via_scrollback = true;
                }
            }
            if !is_agent {
                continue;
            }

            // 3) Classify — piped > title > scrollback > idle.
            let prev = state.tracked.get(&id);
            let mut status = piped.map(|p| p.status).unwrap_or(Status::Unknown);
            let mut via = if piped.is_some() {
                "pipe"
            } else if fp_via_scrollback {
                "scrollback"
            } else {
                "infer"
            };
            // The tool name from the piped payload is stored on the record in
            // handle_pipe_payload; preserve it while a piped status is active.
            let tool = if piped.is_some() {
                prev.and_then(|p| p.tool.clone())
            } else {
                None
            };

            if status == Status::Unknown {
                if let Some(s) =
                    best_classify(&detectors, &info.title, &cmd, scrollback_lines, id, viewport)
                {
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

/// Per-pane fingerprint re-check cadence, in scans. A pane that looked like
/// pi last time is re-checked sooner (it may have changed state) than one that
/// didn't (cheap to keep ignoring a plain shell).
const FP_GAP_HIT: u64 = 3;
const FP_GAP_MISS: u64 = 10;

/// Read a pane's viewport (ANSI stripped). Returns an empty vec on error.
fn read_viewport(pane_id: u32) -> Vec<String> {
    match get_pane_scrollback(PaneId::Terminal(pane_id), false) {
        Ok(contents) => contents
            .viewport
            .into_iter()
            .map(|line| agent::strip_ansi(&line))
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    }
}

/// Run each detector's classifier, returning the first non-`None` status.
/// Tries the title first (cheap), then the viewport. If the caller already
/// read the viewport (for fingerprinting), it's reused instead of re-read.
fn best_classify(
    detectors: &[Box<dyn AgentDetector>],
    title: &str,
    _cmd: &str,
    scrollback_lines: usize,
    pane_id: u32,
    viewport: Option<Vec<String>>,
) -> Option<Status> {
    // Try title-only first (cheap, no host call).
    for d in detectors {
        if let Some(s) = d.classify(title, &[]) {
            return Some(s);
        }
    }

    // Reuse a pre-read viewport, or read one now.
    let viewport = match viewport {
        Some(vp) => vp,
        None => read_viewport(pane_id),
    };
    if viewport.is_empty() {
        return None;
    }

    for d in detectors {
        if let Some(s) = d.classify(title, &viewport) {
            return Some(s);
        }
    }
    let _ = scrollback_lines; // viewport already bounded by pane size
    None
}

/// True if any extra user pattern matches either the command or the title.
fn matches_extra_any(command: &str, title: &str, extra: &[String]) -> bool {
    if extra.is_empty() {
        return false;
    }
    let c = command.to_lowercase();
    let t = title.to_lowercase();
    extra
        .iter()
        .any(|pat| (!c.is_empty() && c.contains(pat)) || (!t.is_empty() && t.contains(pat)))
}
