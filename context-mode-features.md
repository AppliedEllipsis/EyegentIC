# Context-Mode: Complete Feature Reference

**context-mode** is a context-preservation toolkit for AI coding agents. It prevents large command outputs, file contents, and web documentation from flooding the agent's limited context window by routing all data through **sandboxed execution** or a **persistent FTS5 knowledge base**, returning only computed summaries and search snippets to the conversation.

---

## Core Philosophy: Think-in-Code

> The bytes your code processes never enter your conversation memory; only what you `console.log()` does.

Reading a 700 KB log directly costs 700 KB of reasoning capacity. Running code over that log in a sandbox and printing a 3 KB summary preserves 697 KB of capacity for the actual task.

---

## Tool Suite (11 Tools)

### 1. `ctx_execute` — Sandboxed Code Execution

Run code in a sandboxed subprocess. The raw output is auto-indexed into the FTS5 knowledge base; only what your code explicitly prints enters the conversation.

**Supported Languages:** `javascript`, `typescript`, `python`, `shell`, `ruby`, `go`, `rust`, `php`, `perl`, `r`, `elixir`, `csharp`

**Key Parameters:**

| Parameter | Description |
|-----------|-------------|
| `language` | Runtime language |
| `code` | Source code. Must use `console.log`/`print`/`echo` to surface findings. |
| `timeout` | Max execution time in ms (optional) |
| `background` | Keep process alive after timeout (for servers/daemons) |
| `cwd` | Working directory for shell commands |
| `intent` | What you're looking for. Large outputs (>5KB) are auto-indexed by section; use `ctx_search` to retrieve specific sections later. |

**Example:**
```javascript
ctx_execute(language: "javascript", code: `
  const out = require('child_process')
    .execSync('npm test', {encoding:'utf8'});
  const fails = out.split('\n')
    .filter(l => /FAIL/.test(l));
  console.log(fails.length + ' failing tests');
  console.log(fails.slice(0, 30).join('\n'));
`, timeout: 120000)
```

**When to use:** API calls, CLI output, test runs, git analysis, build output, log parsing, Docker/K8s inspection, any command that reads/queries/fetches/logs/tests/builds/diffs.

---

### 2. `ctx_execute_file` — File-in-Sandbox Processing

Read a file into a sandboxed `FILE_CONTENT` variable and run code over it. The file bytes stay in the sandbox — only what you print reaches the conversation.

**Key Parameters:**

| Parameter | Description |
|-----------|-------------|
| `path` | Absolute or relative file path |
| `language` | Runtime language |
| `code` | Code to process `FILE_CONTENT`. Print results with `console.log`/`print`/etc. |
| `timeout` | Max execution time in ms |
| `intent` | What you're looking for (enables section-level indexing for large outputs) |

**Example:**
```python
ctx_execute_file(path: "data.csv", language: "python", code: """
import csv
from collections import Counter
rows = list(csv.DictReader(FILE_CONTENT.splitlines()))
print(f"Records: {len(rows)}")
statuses = Counter(r['status'] for r in rows)
for s, c in statuses.most_common():
    print(f"  {s}: {c}")
""")
```

**When to use:** Analyzing log files, CSV data, JSON configs, large markdown documents, any file where you need to KNOW something without needing to SEE all of it. **Do NOT use for files you intend to edit** — use the Read tool instead.

---

### 3. `ctx_index` — Persistent Knowledge Base Ingestion

Store content in a searchable FTS5 (Full-Text Search 5) knowledge base. Splits markdown by headings, keeps code blocks intact, and persists the raw chunks. The full content lives in storage — retrieve any section on-demand via `ctx_search`.

**Key Parameters:**

| Parameter | Description |
|-----------|-------------|
| `content` | Inline text/markdown. **Use only for small text.** For large data or files use `path`. |
| `path` | File or directory path. Content is read server-side and NEVER enters context. |
| `source` | Label for the indexed content (e.g., `"React useEffect docs"`, `"project:my-app"`) |
| `include` | (Directory only) Glob patterns to include |
| `exclude` | (Directory only) Glob patterns to exclude. Defaults merge: `node_modules`, `.git`, `dist`, `build`, `.next`, `coverage`, `.venv`, `__pycache__`, `.DS_Store` |
| `maxDepth` | (Directory only) Max recursion depth (default: 5) |
| `maxFiles` | (Directory only) Hard cap on files indexed (default: 200) |
| `extensions` | (Directory only) Allowed file extensions (default: `.md`, `.mdx`, `.txt`, `.json`, `.yaml`, `.yml`, `.ts`, `.tsx`, `.js`, `.jsx`, `.py`, `.rs`, `.go`, `.sh`) |
| `respectGitignore` | (Directory only) Apply nearest `.gitignore` (default: true) |
| `followSymlinks` | (Directory only) Follow directory symlinks (default: false) |

**Critical rule:** Always prefer `ctx_index(path: "...")` over `ctx_index(content: large_data)`. The `content` parameter sends bytes through context as a tool parameter — use it only for small inline text you're composing yourself. The `path` parameter reads files server-side.

**When to use:** API documentation, framework guides, README files, migration guides, changelog entries, code examples you need to reference later, MCP tool output requiring multi-query access.

---

### 4. `ctx_search` — Multi-Strategy Knowledge Base Search

Search the unified knowledge base with a multi-strategy ranking pipeline. Queries reach both indexed content AND auto-captured session memory (decisions, errors, plans, blockers — 26 event categories).

**Search Pipeline:** Two parallel matchers run on every query — a Porter-stemming matcher (`"caching"` finds `"cached"`, `"caches"`) and a trigram-substring matcher (`"useEff"` finds `"useEffect"`). Their results are merged via Reciprocal Rank Fusion. Multi-term queries get a proximity-rerank pass. Typos are corrected via Levenshtein distance and re-searched.

**Key Parameters:**

| Parameter | Description |
|-----------|-------------|
| `queries` | Array of search queries. **Batch ALL questions in one call.** Use 2-4 specific technical terms per query. |
| `limit` | Results per query (default: 3) |
| `source` | Filter to a specific indexed source (partial match works) |
| `contentType` | Filter by `"code"` or `"prose"` content type |
| `sort` | `"relevance"` (BM25 ranked, current session) or `"timeline"` (chronological across sessions + auto-memory) |

**Session Memory Source Labels:** `decision`, `error`, `error-resolution`, `blocker`, `plan`, `user-prompt`, `rejected-approach`, `compaction`, plus 18 additional categories.

**Example:**
```javascript
ctx_search({
  queries: ["login form email password validation", "auth middleware token refresh"],
  source: "project:my-app",
  contentType: "code",
  limit: 5
})
```

**When to use:** Recalling indexed content, querying session history, finding past decisions/errors/blockers without rereading raw sources.

---

### 5. `ctx_fetch_and_index` — URL Fetcher + Indexer

Fetch URLs, convert HTML to markdown (JSON is chunked by key paths, plain text indexed directly), and persist in the knowledge base. The raw page bytes never enter conversation.

**Caching:** Every fetch is cached on disk for 24 hours by default. Use `ttl` (milliseconds) to override; `ttl: 0` bypasses cache like `force: true`. Stored content older than 14 days is cleaned up at startup.

**Key Parameters:**

| Parameter | Description |
|-----------|-------------|
| `url` | Single URL to fetch (legacy single-shape) |
| `requests` | Batch: array of `{url, source?}` entries for parallel fetching |
| `source` | Label for indexed content |
| `concurrency` | Max parallel fetches (1-8). Use 4-8 for I/O-bound batches. |
| `force` | Skip cache — fetch fresh |
| `ttl` | Override cache freshness window in ms |

**Example:**
```javascript
ctx_fetch_and_index({
  requests: [
    {url: "https://react.dev/reference/react/useEffect", source: "React useEffect"},
    {url: "https://react.dev/reference/react/useState", source: "React useState"}
  ],
  concurrency: 4
})
// Then search:
ctx_search({queries: ["cleanup function return", "dependency array"], source: "React useEffect"})
```

**When to use:** External documentation, changelogs, API references, library evaluation, multi-URL research. **Not for JavaScript-rendered pages** (SPAs) — requires a headless browser.

---

### 6. `ctx_batch_execute` — Parallel Command Batch

Run multiple commands in ONE call with parallel execution and auto-indexing. Combine with inline queries to get matching results in the same round trip.

**Key Parameters:**

| Parameter | Description |
|-----------|-------------|
| `commands` | Array of `{label, command}` entries |
| `queries` | Search queries to run over indexed output (same round trip) |
| `timeout` | Max execution in ms. With `concurrency>1`, applied per-command. |
| `concurrency` | Max parallel commands (1-8). Use 4-8 for I/O-bound; keep at 1 for CPU-bound. |
| `cwd` | Working directory for all commands |
| `query_scope` | `"batch"` (search only this batch's output) or `"global"` (search entire knowledge base) |

**When to use:** 3+ related commands (multi-issue lookups, git log + diff + blame, multi-file reads, multi-region cloud queries), or when you want to gather AND query in one round trip.

---

### 7. `ctx_stats` — Context Savings Dashboard

Show context consumption statistics for the current session. Displays total bytes returned to context, breakdown by tool, call counts, estimated token usage, and context savings ratio.

- **Read-only** — no reset capability.
- Shows per-tool breakdown: which tools consumed the most context.
- Reports the savings multiplier (e.g., 12.4x savings means 92% of data stayed in sandbox).

**When to use:** Checking how much context was saved, auditing tool efficiency, verifying the savings strategy is working.

---

### 8. `ctx_doctor` — Diagnostics

Run installation diagnostics server-side. Checks:
- Runtimes (Bun/Node detection, availability)
- Hooks configuration (PreToolUse, PostToolUse)
- FTS5 knowledge base integrity
- Plugin registration
- npm and marketplace versions

Returns results with `[OK]`/`[FAIL]`/`[WARN]` prefixes.

**When to use:** Troubleshooting context-mode installation, verifying hooks are active, checking if the knowledge base is functional.

---

### 9. `ctx_purge` — Destructive Content Wipe

**DESTRUCTIVE** — permanently delete indexed content. Cannot be undone.

**Two Scopes:**

| Scope | Parameter | What's Deleted |
|-------|-----------|----------------|
| **Session** | `sessionId: "<uuid>"` | One session's events + per-session FTS5 chunks. Sibling sessions preserved. |
| **Project** | `scope: "project"` | EVERYTHING — FTS5 knowledge base, all session DB rows for all sessions, events markdown, stats file. |

`confirm: true` is always required. There is no undo.

**When to use:** Stale/polluted search results, switching between unrelated projects, isolating a corrupted session.

---

### 10. `ctx_upgrade` — Self-Update

Pull the latest context-mode from GitHub and reinstall. Returns a shell command to execute (build + install + configure hooks).

Process:
1. Pulls latest from GitHub
2. Builds and installs the new version
3. Configures hooks
4. Recommends restarting the session

**When to use:** Updating to the latest context-mode version.

---

### 11. `ctx_insight` — Hosted Analytics Dashboard

Open the context-mode Insight dashboard (`https://context-mode.com/insight`) in the default browser.

Insight is the hosted analytics layer for AI-assisted engineering teams, providing:
- Per-engineer productive rate
- Retry waste analysis
- Blocker detection
- Role-narrowed views (CTO, Engineering Manager, IC, CISO, FinOps, DevOps)

**When to use:** Accessing the hosted analytics dashboard.

---

## Critical Design Patterns

### Pattern 1: Sandboxed Data Workflow (Playwright/Browser)

Always route browser snapshots through file → sandbox:

```
Step 1: browser_snapshot(filename: "/tmp/snap.md")
        → saves to file, returns ~50B confirmation (NOT 135K tokens)

Step 2: ctx_index(path: "/tmp/snap.md", source: "Playwright snapshot")
        → reads file SERVER-SIDE, indexes into FTS5, returns ~80B

Step 3: ctx_search(queries: ["login form email"], source: "Playwright")
        → returns only matching chunks (~300B)

Total context: ~430B instead of 135K tokens (99%+ savings)
```

### Pattern 2: Two-Layer Architecture

`ctx_execute` and `ctx_search` are two separate layers:

```
┌──────────────────────┐      ┌──────────────────────┐
│   ctx_execute        │ ───▶ │   ctx_search         │
│   (capture layer)    │      │   (filter layer)     │
│                      │      │                      │
│   produces full      │      │   queries the        │
│   output into index  │      │   captured index     │
└──────────────────────┘      └──────────────────────┘
```

- **Capture layer** (`ctx_execute`): Run the command in full. Let everything index. Do NOT narrow here.
- **Filter layer** (`ctx_search`): Narrow, query, filter. All narrowing happens here.

Merging the layers (narrowing inside `ctx_execute`) drops data before it reaches the index — lost permanently, with no context-window benefit.

### Pattern 3: File → Index → Search (For Large MCP Tool Outputs)

```
LargeDataTool(filename: "path") → ctx_index(path: "path") → ctx_search(...)
```

Universal pattern for context preservation regardless of source tool (Playwright, GitHub API, AWS CLI, etc.).

---

## Bash Whitelist (Safe for Direct Use)

Only these operations are safe to run directly via Bash — everything else routes through context-mode:

| Category | Commands |
|----------|----------|
| **File mutations** | `mkdir`, `mv`, `cp`, `rm`, `touch`, `chmod` |
| **Git writes** | `git add`, `git commit`, `git push`, `git checkout`, `git branch`, `git merge` |
| **Navigation** | `cd`, `pwd`, `which` |
| **Process control** | `kill`, `pkill` |
| **Package management** | `npm install`, `npm publish`, `pip install` |
| **Simple output** | `echo`, `printf` |

---

## Language Selection Guide

| Situation | Language | Why |
|-----------|----------|-----|
| HTTP/API calls, JSON processing | `javascript` | Native fetch, JSON.parse, async/await |
| Data analysis, CSV, statistics | `python` | csv, statistics, collections, re |
| Shell commands with pipes | `shell` | grep, awk, jq, native tools |
| File pattern matching, find operations | `shell` | find, wc, sort, uniq |

---

## Timeout Recommendations

| Operation | Recommended timeout |
|-----------|---------------------|
| File reading/parsing | 5,000 - 10,000 ms |
| Local computation | 10,000 ms |
| Single API request | 15,000 - 30,000 ms |
| Paginated API calls | 30,000 - 60,000 ms |
| npm install / build | 120,000 ms |
| Full test suite | 120,000 - 300,000 ms |

---

## Anti-Patterns to Avoid

1. **Using `ctx_execute` for <20 line outputs** — Bash is faster and cheaper for tiny outputs.
2. **Forgetting to print output** — code that computes but never `console.log()`s produces empty results.
3. **Using `cat large-file.json` via Bash** — floods context. Use `ctx_execute_file` instead.
4. **Piping through `| head -20`** — loses the rest of the data. Use `ctx_execute` to analyze ALL data and print a summary.
5. **Narrowing inside `ctx_execute`** — merge the layers, lose data. Narrow downstream with `ctx_search`.
6. **Calling `browser_snapshot()` without `filename`** — dumps 135K tokens into context. Always use `filename`.
7. **Using `ctx_index(content: large_data)`** — sends bytes through context twice. Always use `path`.
8. **Re-indexing data already in context** — duplicates context usage. Use directly or save to file first.
9. **Using Bash with inline Python/Node** — if you're embedding `python3 -c` or `node -e`, switch to `language: python` or `language: javascript`.
10. **Not setting timeouts for network operations** — API calls need 15-30s minimum.

---

## Auto-Captured Session Memory (26 Event Categories)

The knowledge base automatically captures session events without any action needed. These are searchable via `ctx_search` with `sort: "timeline"`:

- `decision` — User corrections and preferences
- `error` and `error-resolution` — Past failures and their fixes
- `blocker` — Current blockers
- `plan` — Implementation plans
- `user-prompt` — Original user requests
- `rejected-approach` — Approaches that were ruled out
- `compaction` — Post-compaction session guides
- Plus 18 additional event categories

---

## Subagent Integration

Subagents automatically receive context-mode tool routing via a PreToolUse hook. No manual configuration is needed — just write natural task descriptions.

---

## Summary

context-mode is a defense-in-depth context preservation system. It provides 11 specialized tools that together ensure the agent never wastes capacity on raw data:

- **Sandbox tools** (`ctx_execute`, `ctx_execute_file`) run code over data and return only computed summaries
- **Knowledge base tools** (`ctx_index`, `ctx_search`, `ctx_fetch_and_index`, `ctx_batch_execute`) persist data server-side for on-demand retrieval
- **Utility tools** (`ctx_stats`, `ctx_doctor`, `ctx_purge`, `ctx_upgrade`, `ctx_insight`) manage the system itself

The result: context windows stay lean, reasoning capacity stays high, and agents can reason about data without being overwhelmed by it.
