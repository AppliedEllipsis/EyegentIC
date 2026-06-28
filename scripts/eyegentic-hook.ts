// eyegentic — pipes this pi session's state to the eyegentic zellij plugin.
//
// Auto-installed by the eyegentic plugin (idempotent + version-tagged).
// Safe to delete; eyegentic falls back to scrollback/title inference.
//
// This extension listens to pi's lifecycle events and forwards a compact
// status to zellij via `zellij pipe`. It no-ops when pi isn't running inside
// zellij (no ZELLIJ_PANE_ID).
//
// Status words (matched by eyegentic's Status::from_word):
//   "working"   agent is thinking / running a tool
//   "ready"     agent finished, awaiting your next input
//   "idle"      alive but not doing anything
//   "error"     a tool failed
// The optional `tool` field drives a per-tool icon (⚡ bash / ◉ read / ✎ edit / ◈ web / ⚙ other).

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { spawn } from "node:child_process";

const PIPE_NAME = "eyegentic";

function paneId(): number | null {
  const v = process.env.ZELLIJ_PANE_ID;
  if (!v) return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

function zellijBin(): string | null {
  // zellij isn't running in this process env → nothing to pipe to.
  if (!process.env.ZELLIJ_SESSION_NAME && !process.env.ZELLIJ_PANE_ID) return null;
  return process.platform === "win32" ? "zellij.exe" : "zellij";
}

function pipe(status: string, tool?: string): void {
  const pid = paneId();
  const bin = zellijBin();
  if (pid === null || !bin) return;
  const payload = JSON.stringify({
    pane_id: pid,
    status,
    tool: tool ?? null,
    ts_ms: Date.now(),
  });
  try {
    const child = spawn(bin, ["pipe", "--name", PIPE_NAME, "--", payload], {
      stdio: "ignore",
      windowsHide: true,
    });
    child.on("error", () => {});
    child.unref();
  } catch {
    // Never let telemetry break the agent.
  }
}

export default function (pi: ExtensionAPI): void {
  pi.on("session_start", async () => pipe("idle"));
  pi.on("session_shutdown", async () => pipe("idle"));

  // User typed a new prompt → the agent is about to work.
  pi.on("input", async (event) => {
    if (event.source === "interactive") pipe("working");
  });

  pi.on("agent_start", async () => pipe("working"));
  pi.on("agent_end", async () => pipe("ready"));

  pi.on("tool_call", async (event) => {
    if (event.toolName === "ask_user_question") {
      pipe("needs input", event.toolName);
    } else {
      pipe("working", event.toolName);
    }
  });
  pi.on("tool_execution_end", async (event) => {
    if (event.toolName === "ask_user_question") {
      // Question was answered — agent resumes working.
      pipe("working", event.toolName);
    } else if (event.isError) {
      pipe("error", event.toolName);
    } else {
      pipe("working", event.toolName);
    }
  });
}
