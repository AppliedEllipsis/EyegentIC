# eyegentic

[![License: GLWTS](https://img.shields.io/badge/License-GLWTS-brightgreen.svg)](LICENSE)
[![Made with AI](https://img.shields.io/badge/Co--vibe%20coded%20with-AI%20%F0%9F%96%A4-blueviolet.svg)](#co-vibe-coded-with-ai)
[![Platform: zellij](https://img.shields.io/badge/zellij-WASM%20plugin-orange.svg)](https://zellij.dev)

**Keep an eye on your coding agents — pane status at a glance.** 👁️

A [zellij](https://zellij.dev) plugin that shows your coding-agent panes'
status **at a glance** — *the eye, see,* in agent**ic**.

`eyegentic` is an eye on your agents. It watches the coding-agent panes in your
zellij session and tells you, in one line, which agent is **working**, which is
**ready**, and which is **blocked waiting for you** — so you always know where
your attention is needed.

### Why the name? 👁️

**`eyegentic`** is a portmanteau that reads three ways at once:

- **eye** + **agent** + **-ic** — an *eye* on your coding *agent*s, in the
  *agentic* era.
- Said aloud it's *"eye-see"* → **"I see"** — the plugin lets you *see*
  what every agent is doing.
- The trailing **IC** of "agent**IC**" is literally **"I C"** = **"I see"**.

So the tagline **"The Eye, See, in agentIC"** means: *the eye that sees,
embedded in the agentic age.*

It is inspired by the at-a-glance UX of [herdr](https://herdr.dev) and
[zellaude](https://github.com/ishefi/zellaude), but lives *inside* zellij as a
WASM plugin and is agent-agnostic: it ships with a detector for the
[pi](https://github.com/earendil-works/pi-coding-agent) coding agent and is
built to add more.

## What it shows

Each tracked agent pane is classified into one of:

| Status       | Icon | Color   | Meaning                                  |
|--------------|------|---------|------------------------------------------|
| Working      | ⏳   | amber   | agent is thinking / running a tool       |
| Ready        | ✅   | green   | agent finished, awaiting your next input |
| Needs input  | ❗   | yellow  | agent is asking a question / wants a pick |
| Error        | ❌   | red     | agent errored / failed                    |
| Idle         | ⏸   | dim     | alive but no signal detected             |
| Unknown      | ❔   | gray    | not yet classified                        |

These appear as, in priority order:

1. **a status bar** in the plugin pane — `eyegentic  ❗ web · ⏳ api · ✅ docs   3 agents · 1 working · 1 need input · 0 error` — with optional **elapsed-time** suffixes (`⏳ api 45s`) to spot stuck agents and a **yellow flash** when an agent starts needing input.
2. **icons prefixed onto tab names** — each tab is prefixed with the icon of
   its most attention-worthy agent (`❗ api`).
3. **icons prefixed onto pane frame titles**.
4. *(experimental)* **pane default-color tint** by state.

The bar is also **clickable**: click an agent to focus its pane (if it's
waiting on you) or switch to its tab. Click the **eyegentic** prefix to open
the in-bar **settings menu** and toggle indicators live. If you load eyegentic
in multiple tabs, all instances **sync** so every tab shows the same view.

#### Per-tool icons

When a hook pipes a `tool` name alongside `working`, the bar swaps ⏳ for a
per-tool glyph so you can see *what* the agent is doing:

| tool                    | icon |
|-------------------------|------|
| bash / shell / exec      | ⚡   |
| read / glob / grep / search | ◉   |
| edit / write / patch     | ✎   |
| web / fetch / http       | ◈   |
| task / subagent / spawn  | ⊜   |
| anything else            | ⚙   |

## How it detects state

eyegentic combines three signals; the first that yields a status wins:

1. **Piped** — a pi hook (or any tool) runs
   `zellij pipe --name eyegentic -- '{"pane_id": 123, "status": "working"}'`
   and the plugin records it. This is the most precise and needs no inference.
2. **Title** — the pane's terminal title carries a status token (e.g. pi's
   [pi-dynamic-title](https://github.com/fangwangme/pi-dynamic-title) extension
   writes a braille spinner while running, a `●` when done).
3. **Scrollback** — the plugin reads the pane's viewport via zellij's
   `get_pane_scrollback` and pattern-matches the agent's TUI: a `❯` prompt =
   ready, a spinner / "Working…" = working, a numbered options menu or
   "Do you want to proceed?" = needs input, `error:`/`panic:` = error.

A pane is considered an *agent* when its running command matches a detector
(pi: `pi`, `pi-coding-agent`, `node … pi`) or when one of the signals above
fires.

## Build

You need Rust with a `wasm32-wasi` target.

```bash
rustup target add wasm32-wasi   # or wasm32-wasip1 on newer toolchains
./build.sh                      # builds target/wasm32-wasi/release/eyegentic.wasm
```

`build.sh` auto-detects which `wasm32-wasi*` variant your toolchain has.

## Try it

From the `eyegentic/` folder:

```bash
./build.sh
zellij -l zellij.kdl            # status bar on top, a shell below
```

In the shell pane, launch an agent (`pi`) and watch its icon update as it
works and finishes.

## Install permanently

The easiest way is to point your layout at a [release](https://github.com/AppliedEllipsis/EyegentIC/releases)
artifact — no Rust toolchain needed:

```kdl
default_tab_template {
    pane size=1 borderless=true {
        plugin location="https://github.com/AppliedEllipsis/EyegentIC/releases/latest/download/eyegentic.wasm"
    }
    children
}
```

Or build from source and load the local file:

```kdl
layout {
    pane size=1 borderless=true {
        plugin location="file:/path/to/eyegentic.wasm" {
            poll_interval "1.0"
            status_bar "true"
            rename_tabs "true"
            rename_panes "true"
            pane_tint "false"
        }
    }
    pane
}
```

> **Windows note:** use `file:D:/path/eyegentic.wasm` (no slash after `file:`).
> The `file:/D:/...` form the docs show fails with OS error 123 on Windows.

## Configuration

All keys are optional (defaults shown):

| key                  | default | meaning                                            |
|----------------------|---------|----------------------------------------------------|
| `poll_interval`      | `1.0`   | seconds between status scans (min `0.2`)           |
| `status_bar`         | `true`  | draw the one-line status bar in the plugin pane    |
| `rename_tabs`        | `true`  | prefix tab names with the representative status icon |
| `rename_panes`       | `true`  | prefix pane frame titles with the agent's icon     |
| `pane_tint`          | `false` | tint each agent pane's default colors by state     |
| `scrollback_lines`   | `14`    | trailing viewport lines to inspect                 |
| `extra_agent_patterns` | `""`  | comma-separated extra command substrings to treat as agents |
| `elapsed_threshold`  | `30`    | seconds before an elapsed-time suffix appears      |
| `working_stale_secs` | `600`   | demote a stale piped `working` to idle after this  |
| `flash`              | `brief` | needs-input flash: `off` / `brief` / `persist`      |
| `elapsed_time`       | `true`  | show elapsed-time suffixes in the bar              |
| `auto_install_hook`  | `true`  | auto-install the pi hook extension on first load    |

The toggleable subset (`status_bar`, `rename_tabs`, `rename_panes`,
`pane_tint`, `elapsed_time`, `flash`) is also live-editable from the in-bar
**settings menu** (click the **eyegentic** prefix) and persisted to
`~/.config/zellij/plugins/eyegentic.json`, synced across all plugin instances.

> **About `pane_tint` / "border colors":** zellij's plugin API exposes
> `set_pane_color` (a pane's *default* fg/bg) but no border-only color, so
> this tints the whole pane and is off by default. The visible indicators are
> the status bar and the icon prefixes.

## Auto-installed pi hook (zero setup)

On first load — after you grant permissions — eyegentic **automatically**
writes a small TypeScript extension to
`~/.pi/agent/extensions/eyegentic/index.ts`. pi auto-discovers extensions
there, so the next time you start `pi` in a zellij pane it will begin piping
precise status (`working` / `ready` / `idle` / `error`, plus the current tool
name) into eyegentic. No manual wiring required.

The install is **idempotent** (a version tag skips re-writing when current)
and **backs up** an older file to `index.ts.bak` before overwriting. Removing
the file is always safe — eyegentic falls back to scrollback/title inference.

- **Disable it:** set `auto_install_hook "false"` in the plugin config.
- **Uninstall the hook:** `rm -rf ~/.pi/agent/extensions/eyegentic`.
- **What it does:** on `agent_start`/`tool_call`/`tool_execution_end` it pipes
  `working` (+ tool name); on `agent_end` it pipes `ready`; on a tool error it
  pipes `error`; on `session_start`/`session_shutdown` it pipes `idle`. It no-ops
  when `pi` isn't running inside zellij.

> The hook calls `zellij pipe --name eyegentic -- '<json>'` over the CLI, so
> it needs the `ReadCliPipes` permission (requested automatically). The JSON
> is `{"pane_id":N,"status":"...","tool":"...","ts_ms":N}`; `ts_ms` lets
> eyegentic drop out-of-order events from parallel hook subprocesses.

## Permissions

On first load, eyegentic requests:

- `ReadApplicationState` — observe tabs/panes and the input mode
- `ReadPaneContents` — read pane viewports for scrollback inference
- `ChangeApplicationState` — rename tabs/panes and tint colors
- `RunCommands` — load/save the settings file and auto-install the hook
- `ReadCliPipes` — receive `zellij pipe --name eyegentic` from the hook
- `MessageAndLaunchOtherPlugins` — sync state across plugin instances

Grant them when zellij prompts. If denied, the status bar explains what's
missing.

## Architecture

```
src/
  lib.rs        ZellijPlugin impl, event dispatch, mouse + pipe routing
  config.rs     load-time config parsing
  settings.rs   persisted, runtime-toggleable settings (serde)
  state.rs      in-memory state: tracked panes, piped statuses, flash, sync
  status.rs      Status enum + icons/colors/attention ranking + per-tool icons
  agent/
    mod.rs       AgentDetector trait + shared title/scrollback heuristics
    pi.rs        pi detector (command match + classify)
  detect.rs      walk the manifest, classify each agent pane
  indicators.rs  apply tab/pane rename + pane tint
  render.rs      the status bar (flash, elapsed, click regions, settings menu)
  installer.rs   auto-install the pi hook extension
scripts/
  eyegentic-hook.ts  the auto-installed pi extension (piped status source)
```

To support another agent, implement `AgentDetector` in `src/agent/` and add it
to `detectors()` in `src/agent/mod.rs`.

## Status

Early scaffold — detection heuristics are tunable guesses and will be refined
against real agent output. The scrollback parser is intentionally conservative
(false negatives over false positives). Feedback and detector PRs welcome.

## License

GLWTS (Good Luck With That Shit) Public License — See [LICENSE](LICENSE) for
details.

You can do whatever the fuck you want with this software at your OWN RISK. The
author has no fucking clue what the code does, and you can never track them down
to blame them.

---

## Support This Project ❤️

If you find this useful, then please support its continued development:

### Crypto Donation

If you'd prefer to donate directly via cryptocurrency, you can send Bitcoin to:

**`bc1q8nrdytlvms0a0zurp04xwfppflcxwgpyrzw5hn`**

Thank you for supporting free and open source software! 🙏

---

## Co-vibe coded with AI

Built with human creativity enhanced by artificial intelligence. 🖤
