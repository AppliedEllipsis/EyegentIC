# Eyegentic Audit — Performance, Features, Bugs, Architecture

**Date:** 2026-06-28 | **Version:** 0.1.0 | **Commit:** 2a7cf6c

---

## 1. Performance

### 1.1 Hot-path allocations (every tick)

The `Timer` → `tick()` path fires every `poll_interval` seconds (default 1.0s). These allocations happen on every tick:

| Location | Allocation | Severity |
|---|---|---|
| `detect::detect_all` | Clones entire `PaneManifest` (BTreeMap) | Medium |
| `detect::detect_all` | Clones every `PaneInfo.terminal_command` via `.clone().unwrap_or_default()` | Low |
| `detect::detect_all` | Rebuilds fresh `BTreeMap<u32, PaneRecord>` from scratch | Medium |
| `indicators::apply` | Clones `state.tabs` (Vec) for `rename_tabs` | Low |
| `indicators::apply` | Clones each pane title (`r.title.clone()`) | Low |
| `status::strip_our_prefix` | Allocates `format!("{} ", icon)` for each of 7 icons per call | Low |
| `classify_scrollback` | Joins viewport lines into a String | Low (throttled) |
| `pi_fingerprint_score` | Joins 24 lines with newlines into a String | Low (throttled) |

**Assessment:** For a WASM plugin monitoring <20 panes at 1s intervals, the overhead is negligible. The design is clean, single-threaded, and allocation-heavy but allocation-light in absolute terms (~1-5KB per tick). Not a real performance problem.

### 1.2 Render path

| Location | Allocation | Severity |
|---|---|---|
| `render_bar` | `String::with_capacity(cols * 6)` | Low |
| `render_agents` | Collects tracked into Vec, sorts every render | Low |
| `format_elapsed` | Per-agent String allocation | Low |
| `visible_width` | O(chars) scan over full bar string every render | Low |

**Assessment:** The render path is efficient. The bar string is at most a few hundred bytes. Three passes over agents (count waiting, render segments, summary) could be unified but it's not worth the complexity for <10 agents.

### 1.3 WASM build profile (✅ Good)

```toml
[profile.release]
opt-level = "z"       # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Single codegen unit for better inlining
panic = "abort"        # No unwind machinery
strip = true           # Strip symbols
```

This is the optimal release profile for a Zellij WASM plugin. No issues.

### 1.4 Host calls (expensive)

Every `Timer` tick triggers:
- `get_pane_scrollback` — only for newly-detected agent panes (fingerprinted), throttled every 3-10 scans
- `set_pane_color` — only when status changes
- `rename_tab` / `rename_terminal_pane` — only when icon changes
- `probe::probe_all` (new) — 3 host calls per pane, every 5th scan, only when `probe = true`

**Assessment:** Host calls are well-throttled. The probe module correctly limits itself to every 5th scan. No unnecessary Zellij API calls in the hot path.

---

## 2. Feature Completeness

### 2.1 What works (✅)

| Feature | Status |
|---|---|
| Three-signal detection (pipe, title, scrollback) | ✅ |
| pi detection via π glyph in terminal title (no hook needed) | ✅ |
| Auto-installed pi hook (zero-config piping) | ✅ |
| Status bar with agent status, icons, elapsed time | ✅ |
| Tab name icon prefixes | ✅ |
| Pane frame title icon prefixes | ✅ |
| Pane color tinting by status | ✅ |
| Multi-tab sync (inter-plugin messaging) | ✅ |
| Click-to-focus / click-to-switch-tab | ✅ |
| Needs-input flash (brief/persist/off) | ✅ |
| In-bar settings menu (click eyegentic prefix) | ✅ |
| Persisted settings to disk | ✅ |
| Stale piped status cleanup | ✅ |
| Out-of-order pipe event guard (ts_ms) | ✅ |
| Per-tool icons (⚡ bash, ◉ read, ✎ edit, ◈ web, ⊜ subagent) | ✅ |
| Mode + session display in status bar | ✅ |
| Permission cache seeding (first-run freeze workaround) | ✅ |
| Backwards-compat `border_color` → `pane_tint` alias | ✅ |
| Probe diagnostic (new, uncommitted) | ✅ |
| CI release workflow | ✅ |

### 2.2 Missing or incomplete

| Gap | Impact |
|---|---|
| **No detector for non-pi agents** (Cline, Codex, Aider, etc.) | The `AgentDetector` trait is designed for extension but only pi is implemented. |
| **No zellij_pipe-based detection** for agents that use zellij's native pipe API | Agents that pipe status directly (not via CLI) aren't handled. |
| **`probe.rs` not wired into detection** | The OS queries are logged but not used for agent detection. The comment says "wire useful ones into detect.rs and delete this module." |
| **No "agent exited" detection** | If an agent pane exits (shell returns to prompt), it stays tracked as its last-known status. `PaneInfo.exited` is available but not checked in `detect_all`. |
| **No auto-reload on hook update** | If the hook TS is updated, existing pi sessions keep running the old version until pi restarts. |
| **No scrollback-based idle→agent transition** | A pane that starts as a shell then launches pi will only be detected on the next fingerprint check (every 3-10 scans). Piped events provide instant detection, but without the hook it's delayed. |

---

## 3. Bugs & Edge Cases

### 3.1 Confirmed bugs

| # | File | Bug | Severity | Fix |
|---|---|---|---|---|
| 1 | `state.rs` | `is_flash_bright` calls `unix_now_ms()` twice, which can return different values on the 250ms boundary — the separate calls could disagree on brightness parity, causing a one-tick flicker. | Cosmetic | Capture `unix_now_ms()` once at the top. |
| 2 | `status.rs` | `strip_our_prefix` allocates `format!("{} ", icon)` for every icon on every call — called per-agent per-scan. | Low (tiny strings) | Use string slicing: check `name.len() > icon.len() + 1` and `&name[..icon.len()+1] == format!("{} ", icon)`. Or store preformatted `"icon "` strings in `OUR_ICONS`. |
| 3 | `render.rs` | `render_bar` takes `rows` parameter but ignores it (`_rows`). If Zellij gives the plugin 2+ rows, the bar only renders on row 1, leaving garbage on row 2. | Low (plugin is 1 row) | Either assert or clear the full area. |
| 4 | `render.rs` | `visible_width` counts Unicode chars as width 1 — CJK/wide chars would be miscounted. | Very low (no CJK in output) | Not worth fixing unless wide chars ever appear. |

### 3.2 Design edge cases

| # | Where | Issue | Likelihood |
|---|---|---|---|
| 1 | `detect.rs` | `viewport` is conditionally populated: if fingerprinting succeeded, it's `Some`; if `is_agent` was already true from cheap signals, it's `None`. `best_classify` receives `None` in that case and re-reads the viewport — a wasted host call. | Medium | Always fingerprint or always pass through. |
| 2 | `agent/mod.rs` | `classify_scrollback` checks for `❯` on the last non-empty line — any shell with starship/powerline using `❯` would classify as `Ready` once a pane is already classified as an agent. | Low (gated by fingerprint: needs >= 2 signals) |
| 3 | `agent/mod.rs` | `has_numbered_options` checks for `"1. "` and `"1) "` — a line like `version 1.0 released` would be a false positive. | Very low |
| 4 | `agent/pi.rs` | `matches_command` uses `c.contains("/pi")` — `/path/to/spinner` would match. | Very low |
| 5 | `state.rs` | `handle_pipe_payload` uses `ts_ms > 0` for out-of-order guard — if two parallel hook invocations both send `ts_ms: 0` (or omit it), the second one can be wrongly dropped. | Low (hook always sends `ts_ms`) |
| 6 | `state.rs` | `merge_sync` keyed on `last_ts_ms` — if two instances send sync at the same ms, the last received wins, potentially overwriting a `NeedsInput` with `Working`. | Very low (1ms precision) |
| 7 | `log.rs` | Opens/closes `/host/eyegentic.log` per `logln!` call — fine for occasional use but many I/O syscalls if verbose. | Very low |

---

## 4. Architecture Quality

### 4.1 What's good

- **Hybrid detection with clear priority:** Piped > Title > Scrollback — the first match wins, and each level is more expensive but more precise.
- **Agent-agnostic by design:** The `AgentDetector` trait cleanly separates per-agent heuristics from the generic detection loop.
- **Defense in depth:** pi fingerprinting requires >= 2 of 5 signals, so no single false positive (like `❯` alone) can pass.
- **Stale cleanup:** Piped `Working` demotes to idle after `working_stale_secs` (600s default), preventing stuck agents.
- **Out-of-order guard:** `ts_ms` prevents race conditions from parallel hook subprocesses.
- **First-run freeze fix:** Permission cache seeding is a thoughtful workaround for the zellij bug.
- **Idempotent operations:** Indicators are only applied when they actually change, avoiding redundant host calls.
- **Single-threaded model:** Clean, no synchronization concerns.
- **`include_str!` for hook script:** Embeds at compile time — no runtime file I/O for the hook content.

### 4.2 Architecture concerns

| Concern | Detail |
|---|---|
| **`detect_all` rebuilds everything from scratch** | A differential approach would reduce allocations but add complexity. The current design is "simple and correct" — acceptable at this scale. |
| **`scrollback_lines` is dead code** | Declared in `Config`, passed through `detect_all` → `best_classify`, but `get_pane_scrollback` returns full viewport. The parameter has no effect. Either make it work or remove it. |
| **`PaneRecord::via` is `&'static str`** | Works because all assignments use static literals, but `SyncRecord::via` is a `String` — the serialization roundtrip silently converts statics to owned strings. Fine, but the type mismatch is confusing. |
| **No structured error type** | All errors are handled with `if let Err(e)` and logged or ignored. A custom error enum would make error flow more visible. |
| **`probe.rs` intent is unclear** | It's marked as throwaway but exists as a committed module. Should either be deleted or promoted to actual detection logic. |

---

## 5. Security

| Vector | Assessment |
|---|---|
| **Shell injection via `extra_agent_patterns`** | Low risk: patterns are used in substring matching, not shell execution. |
| **Shell injection via settings file path** | Low risk: `save_settings` uses `$HOME/.config/zellij/plugins/eyegentic.json` — no user input in path. |
| **Shell injection via hook installer** | Low risk: heredoc with single-quoted delimiter prevents shell expansion. The script content is compiled-in. |
| **JSON parsing of piped messages** | `serde_json` is robust. Unknown fields are ignored. Malformed JSON is caught. |
| **Log file path** | `/host/eyegentic.log` is written in Zellij's launch directory — could be a symlink attack if the directory is world-writable (unlikely for a dev machine). |

**No critical security issues found.**

---

## 6. Recommendations

### High priority

1. **Handle pane exit** — Check `PaneInfo.exited` in `detect_all` and remove exited panes from `tracked`.
2. **Remove dead `scrollback_lines` parameter** — Either implement bounded viewport reads or delete the config key.

### Medium priority

3. **Add a basic shell/terminal detector** — Even a stub that just shows "unknown agent" would make the plugin useful for non-pi users.
4. **Wire probe results into detection** — `get_pane_running_command` is likely more reliable than `PaneInfo.terminal_command` for detecting wrapped/renamed agents. The probe module already gathers this data.
5. **Document the extension point** — Add a concrete example in `src/agent/` showing how to add a new detector (e.g., a commented-out `claude.rs` stub).

### Low priority

6. **Fix double `unix_now_ms()` call** in `is_flash_bright`.
7. **Pre-format icon prefixes** in `OUR_ICONS` to avoid `strip_our_prefix` allocations.
8. **Consider log rotation** or size check for `eyegentic.log`.
9. **Add `visible_width` CJK support** if internationalization is planned.

---

## 7. Summary

**Eyegentic is well-built for a 0.1.0 plugin.** The core detection pipeline is sound: three signals with clear priority, agent-agnostic trait design, and defense-in-depth fingerprinting. Performance is not a real concern for its scale (<20 panes, 1s poll). The biggest functional gap is the lack of pane-exit handling and the dead `scrollback_lines` parameter. No critical bugs were found — the issues listed are cosmetic or edge cases that rarely trigger in practice.

**Verdict:** Ship it. It does what it says on the tin.
