# Changelog

All notable changes to **EyegentIC** are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **Second tab never got an icon.** `rename_tab` is 1-based (index `0` is
  treated as `1`) but the code passed the 0-based `tab.position` directly, so
  the second tab's rename collided with the first and was silently dropped.\  Now `tab.position + 1` is passed, mirroring `switch_tab_to`. Every tab —
  including ones opened after the plugin loaded — gets its own representative
  icon.
- **Icon crowding the title.** Status glyphs are double-width emoji, so a
  single space between icon and name read as cramped. Bumped to two spaces in
  the tab-name prefix, the pane-frame prefix, and the status-bar segment for a
  consistent gap everywhere.

## [0.1.0] — 2025-06-27

First public release.

### Fixed
- Build as a WASI **command (bin) crate** (`src/main.rs`, no `[lib]`), not a
  `cdylib`. The `register_plugin!` macro emits `fn main()`, so the toolchain
  must produce a `_start` entry point for zellij to instantiate the plugin.
  A `cdylib` has no `_start` and fails to load with *"could not find exported
  function"*.
 A zellij WASM plugin that keeps an eye on your
coding-agent panes and shows, at a glance, which agent is **working**,
**ready**, **needs input**, or **errored** — *the eye, see, in agentIC.*

### Added

**Status detection (three signals, first match wins)**
- **Piped** — agents (or any tool) report precise state via
  `zellij pipe --name eyegentic -- '{"pane_id":N,"status":"working"}'`.
- **Title** — parses the pane terminal title (braille spinner = working,
  `●` = ready, status words after `:`/`|`/`·`).
- **Scrollback** — reads the pane viewport and matches the agent TUI
  (`❯` prompt = ready, spinner / "Working…" = working, numbered options /
  "Do you want to proceed?" = needs input, `error:`/`panic:` = error).

**Indicators**
- One-line **status bar** in the plugin pane, agents sorted by attention.
- **Tab-name** icon prefixes (the most attention-worthy agent per tab).
- **Pane frame title** icon prefixes.
- *(experimental)* **pane color tint** by state (`pane_tint`, off by default).

**Inspired by [zellaude](https://github.com/ishefi/zellaude)**
- **Auto-install pi hook** — on first load, writes a version-tagged,
  idempotent pi extension to `~/.pi/agent/extensions/eyegentic/index.ts`
  that pipes precise agent state. Opt out with `auto_install_hook "false"`.
  Backs up an older version before overwriting.
- **Per-tool icons** — a richer piped payload (`{"tool":"bash"}`) shows
  `⚡` bash · `◉` read/grep/glob · `✎` edit/write · `◈` web · `⊜` subagent · `⚙` other.
- **Out-of-order event guard** — piped events carry `ts_ms`; stale events that
  race through parallel hook subprocesses are dropped.
- **Stale cleanup** — a piped `working` with no follow-up decays to `idle`
  after `working_stale_secs`, letting inference resume.
- **Elapsed time** — agents in a non-idle state for longer than
  `elapsed_threshold` show how long (`⏳ api 45s`), to spot stuck sessions.
- **Multi-tab sync** — plugin instances share state via inter-plugin messages,
  so every tab shows one unified view.
- **Click-to-focus** — click an agent in the bar to focus its pane (if it's
  waiting on you) or switch to its tab.
- **Needs-input flash** — the bar pulses yellow when an agent needs input
  (`flash`: `brief` / `persist` / `off`).
- **Settings menu** — click the `eyegentic` prefix to toggle indicators live;
  settings persist to `~/.config/zellij/plugins/eyegentic.json` and sync across
  instances.
- **Mode + session** — the bar shows the zellij session name and current input
  mode (NORMAL / LOCKED / PANE / …).

**Packaging**
- **CI release workflow** — pushing a `v*` tag builds the wasm and attaches it
  to a GitHub release, enabling one-line install via
  `https://github.com/AppliedEllipsis/EyegentIC/releases/latest/download/eyegentic.wasm`.

### Notes
- Agent-agnostic by design: ships with a detector for
  [pi](https://github.com/earendil-works/pi-coding-agent); add more via the
  `AgentDetector` trait in `src/agent/`.
- **Windows `file:` paths:** use `file:D:/path/to/eyegentic.wasm` (no slash
  after `file:`); the documented `file:/D:/…` form fails on Windows.
- Permissions requested: `ReadApplicationState`, `ReadPaneContents`,
  `ChangeApplicationState`, `RunCommands`, `ReadCliPipes`,
  `MessageAndLaunchOtherPlugins`.

[0.1.0]: https://github.com/AppliedEllipsis/EyegentIC/releases/tag/v0.1.0
