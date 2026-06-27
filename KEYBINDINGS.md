# eyegentic — Keybinds & Layout Reference

This documents the **custom zellij keybinds** used on this host
(`C:\Users\User\AppData\Roaming\Zellij\config\config.kdl`) and the **dev layout**
shipped with eyegentic (`zellij.kdl`).

> ⚠️ This config uses `keybinds clear-defaults=true` — **all** of zellij's stock
> default keybinds are wiped. **Only** the bindings below exist. The mode-switch
> modifier is **`Ctrl Shift`** (not zellij's stock `Ctrl`), matching the
> user's preferred convention.

---

## TL;DR

- **Mode switch modifier:** `Ctrl Shift` (everywhere)
- **Move focus without a mode:** `Alt h j k l` (or arrows) — works in normal mode
- **Quit zellij:** `Ctrl Shift q`
- **Lock (passthrough) mode:** `Ctrl Shift g` → then `Ctrl Shift g` to exit
- **Open eyegentic as a floating plugin:** `Ctrl Shift m`
- **Back to normal:** `Esc` or `Enter` (from any sub-mode)

---

## Mode switchers (available from any mode)

| Key | Switches to mode |
|---|---|
| `Ctrl Shift g` | **locked** (passthrough — keys go to the app, not zellij) |
| `Ctrl Shift p` | **pane** |
| `Ctrl Shift t` | **tab** |
| `Ctrl Shift n` | **resize** |
| `Ctrl Shift h` | **move** |
| `Ctrl Shift s` | **scroll** |
| `Ctrl Shift o` | **session** |
| `Ctrl Shift m` | launch **eyegentic** as a floating plugin |
| `Ctrl Shift q` | quit zellij |
| `Ctrl b` | **tmux** mode |
| `Esc` / `Enter` | return to **normal** mode |

---

## Normal mode (default)

The only keys that do something *without* entering a mode are the **`Alt`-prefixed**
ones (see [Global `Alt` shortcuts](#global-alt-shortcuts) below). Everything else
requires switching into a mode first with `Ctrl Shift <letter>`.

---

## pane mode — enter with `Ctrl Shift p`

Manage panes in the current tab.

| Key | Action |
|---|---|
| `h` `j` `k` `l` / arrows | move focus |
| `n` | new pane (split) |
| `d` | new pane **down** |
| `r` | new pane **right** |
| `s` | new pane **stacked** |
| `p` | switch focus to next pane |
| `f` | toggle fullscreen |
| `z` | toggle pane frames |
| `e` | toggle embed / floating |
| `w` | toggle floating panes |
| `i` | toggle pane pinned |
| `c` | rename pane (enters renamepane input) |
| `Ctrl Shift p` | back to normal |

### renamepane input (after `c`)

| Key | Action |
|---|---|
| `Esc` | undo rename, back to pane mode |
| `d` | detach session |

---

## tab mode — enter with `Ctrl Shift t`

Manage tabs in the session.

| Key | Action |
|---|---|
| `h` `j` `k` `l` / arrows | prev / next tab |
| `1` – `9` | go to tab N |
| `tab` | toggle to last-used tab |
| `n` | new tab |
| `x` | close tab |
| `r` | rename tab (enters renametab input) |
| `s` | toggle active-sync on the tab |
| `b` | break pane into a new tab |
| `[` | break pane left |
| `]` | break pane right |
| `Ctrl Shift t` | back to normal |

### renametab input (after `r`)

| Key | Action |
|---|---|
| `Esc` | undo rename, back to tab mode |
| `Ctrl c` | back to normal |

---

## resize mode — enter with `Ctrl Shift n`

| Key | Action |
|---|---|
| `h` `j` `k` `l` / arrows | increase size in that direction |
| `H` `J` `K` `L` | **decrease** size in that direction |
| `+` / `=` | increase overall |
| `-` | decrease overall |
| `Ctrl Shift n` | back to normal |

---

## move mode — enter with `Ctrl Shift h`

Move the *focused pane itself* (not the focus).

| Key | Action |
|---|---|
| `h` `j` `k` `l` / arrows | move pane in that direction |
| `n` / `tab` | move pane |
| `p` | move pane backwards |
| `Ctrl Shift h` | back to normal |

---

## scroll mode — enter with `Ctrl Shift s`

Browse pane scrollback.

| Key | Action |
|---|---|
| `j` `k` / arrows | scroll down / up |
| `u` | half-page up |
| `d` | half-page down |
| `Ctrl f` / `Ctrl b` | page down / up |
| `e` | edit scrollback (opens `$EDITOR`) |
| `s` | enter search (→ `entersearch` input) |
| `Alt h/j/k/l` | move focus (or tab at edge) and exit |
| `Ctrl Shift s` | back to normal |

### search input (after `s`)

| Key | Action |
|---|---|
| `Enter` | run search (→ **search** mode) |
| `Esc` / `Ctrl s` | back to scroll mode |

### search mode (after running a search)

| Key | Action |
|---|---|
| `n` | search down (next match) |
| `p` | search up (prev match) |
| `c` | toggle case sensitivity |
| `o` | toggle whole-word |
| `w` | toggle wrap |

---

## session mode — enter with `Ctrl Shift o`

Each key launches a zellij builtin as a **floating** plugin, focused on the
current tab, then returns to normal mode.

| Key | Opens |
|---|---|
| `a` | **About** |
| `c` | **Configuration** |
| `l` | **Layout manager** |
| `p` | **Plugin manager** |
| `s` | **Share** (session sharing) |
| `w` | **Session manager** (detach/attach) |
| `Ctrl Shift o` | back to normal |

---

## tmux mode — enter with `Ctrl b`

For tmux muscle memory.

| Key | Action |
|---|---|
| `"` | split down |
| `%` | split right |
| `c` | new tab |
| `n` / `p` | next / previous tab |
| `o` | focus next pane |
| `z` | toggle fullscreen |
| `[` | enter scroll mode |
| `,` | rename tab |
| `space` | next swap layout |
| `h` `j` `k` `l` / arrows | move focus |
| `Ctrl b` | send a literal `Ctrl b` to the app and exit |

---

## locked mode — enter with `Ctrl Shift g`

All keys pass through to the application. zellij does nothing.

| Key | Action |
|---|---|
| `Ctrl Shift g` | exit locked → normal |

---

## Global `Alt` shortcuts

These work **in normal mode without entering any sub-mode** — the fastest way to
get around.

| Key | Action |
|---|---|
| `Alt h` `j` `k` `l` / arrows | move focus (or switch tab at the edge) |
| `Alt n` | new pane |
| `Alt f` | toggle floating panes |
| `Alt p` | toggle pane in group |
| `Alt Shift p` | toggle group marking |
| `Alt i` | move tab **left** |
| `Alt o` | move tab **right** |
| `Alt [` | previous swap layout |
| `Alt ]` | next swap layout |
| `Alt +` / `Alt =` | increase size |
| `Alt -` | decrease size |

---

## eyegentic-specific: `Ctrl Shift m`

`Ctrl Shift m` launches eyegentic itself as a **floating** plugin
(`LaunchOrFocusPlugin "eyegentic" { floating true; move_to_focused_tab true }`),
so you can pop the agent-status view on demand without reloading the layout.

The plugin location is registered in `config.kdl` as:

```kdl
eyegentic location="file:C:/Users/User/AppData/Roaming/zellij/plugins/eyegentic.wasm"
```

---

## The dev layout (`zellij.kdl`)

Run with:

```bash
./build.sh          # builds + copies eyegentic.wasm into ./
zellij -l zellij.kdl
```

### Why `zellij -l` "loses the wrapper"

Plain `zellij` (no `-l`) loads zellij's **default layout**, which wraps every
tab in a `default_tab_template` that loads `zellij:tab-bar` (top, 1 line) and
`zellij:status-bar` (bottom, 2 lines — the context-sensitive keybind hints).

`zellij -l zellij.kdl` **replaces** that default layout with this file's
`layout {}` block. The original dev layout only declared the eyegentic plugin +
a bare `pane`, so the tab-bar / status-bar wrapper dropped out — which is why
the keyboard-info bar disappeared.

### Current layout (fixed)

The dev layout now carries its own `default_tab_template` so the wrapper
returns, while keeping eyegentic on top:

```kdl
layout {
    default_tab_template {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        children
        pane size=2 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
    tab name="eyegentic" {
        pane size=1 borderless=true {
            plugin location="file:eyegentic.wasm" {
                poll_interval "1.0"
                status_bar "true"
                rename_tabs "true"
                rename_panes "true"
                pane_tint "false"
            }
        }
        pane
    }
}
```

Result, top to bottom:

1. `zellij:tab-bar` — tab strip (1 line)
2. **eyegentic** — agent-status bar (1 line)
3. shell pane — launch `pi` here
4. `zellij:status-bar` — the keyboard-hints wrapper (2 lines)

> **New tabs** (`Ctrl Shift t` then `n`) inherit the `default_tab_template`, so
> they get the tab-bar + status-bar wrapper, but **not** the eyegentic bar
> (that lives only in the first tab). If you want eyegentic on every tab, move
> its `pane` block into the `default_tab_template`, above `children`.

---

## Registered plugins (from `config.kdl`)

```kdl
plugins {
    ...
    plugin-manager location="zellij:plugin-manager"
    status-bar     location="zellij:status-bar"
    tab-bar        location="zellij:tab-bar"
    eyegentic      location="file:C:/Users/User/AppData/Roaming/zellij/plugins/eyegentic.wasm"
}
```

`status-bar` and `tab-bar` are the builtins that provide the on-screen hints.
eyegentic is loaded from the local release artifact.

---

## Windows / host notes

- **Modifier convention:** `Ctrl Shift` is this host's mode-switch prefix
  (matches the PasteThrough / pi keybinding convention), **not** zellij's
  stock `Ctrl`-only prefix. Don't expect `Ctrl p` to do anything here — it's
  `Ctrl Shift p`.
- **Bracketed paste:** zellij 0.44.3 on native Windows mangles multi-line paste
  (each newline fires as Enter). See the PasteThrough subproject for the Ctrl+V
  workaround. (Tracked: zellij-org/zellij #4885, #3865.)
- **WASM plugin path:** use `file:D:/path/eyegentic.wasm` (no slash after
  `file:`). The docs' `file:/D:/...` form fails with OS error 123 on Windows.
- **eyegentic dev path:** the dev layout uses `file:eyegentic.wasm` (relative
  to the cwd you launch `zellij -l` from, i.e. the `eyegentic/` folder).
