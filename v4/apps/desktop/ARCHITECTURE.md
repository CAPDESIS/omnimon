# OmniMon v5 Desktop — Frontend Architecture

## Overview

The desktop app is a **Tauri 2 + Svelte 5** application. Rust owns all system
access (processes, memory, kill signals). The Svelte frontend is a thin,
reactive view layer that never touches the OS directly.

```
┌─────────────────────────────────────────────────────────┐
│                     macOS / Linux / Windows              │
│  ┌───────────────────────────────────────────────────┐  │
│  │              Rust Backend (src-tauri/)             │  │
│  │                                                   │  │
│  │  macmon_core::watcher ──▶ SystemStats (cached)    │  │
│  │  macmon_core::metrics ──▶ ProcessMemoryEntry[]    │  │
│  │  macmon_core::killer  ──▶ kill_process_safe()     │  │
│  │  sysinfo::System      ──▶ CPU / exe / uptime      │  │
│  │                                                   │  │
│  │  ┌─────────────────────────────────────────────┐  │  │
│  │  │         Tauri IPC Commands (lib.rs)         │  │  │
│  │  │                                             │  │  │
│  │  │  get_metrics()  → Metrics { procs, stats }  │  │  │
│  │  │  kill_process(pid)  → bool                  │  │  │
│  │  │  kill_processes(pids)  → killed[]           │  │  │
│  │  └──────────────────┬──────────────────────────┘  │  │
│  └─────────────────────┼─────────────────────────────┘  │
│                        │ JSON over IPC                   │
│  ┌─────────────────────▼─────────────────────────────┐  │
│  │           Svelte Frontend (src/)                  │  │
│  │                                                   │  │
│  │  stores/processes.ts ──▶ invoke("get_metrics")    │  │
│  │       │                                           │  │
│  │       ▼                                           │  │
│  │  ┌─────────┐  ┌──────────┐  ┌───────────────┐   │  │
│  │  │processes│  │  stats   │  │ selectedPids  │   │  │
│  │  │writable │  │ writable │  │   writable    │   │  │
│  │  └────┬────┘  └────┬─────┘  └───────┬───────┘   │  │
│  │       │            │                │            │  │
│  │       ▼            │                ▼            │  │
│  │  ┌─────────┐       │         ┌────────────┐     │  │
│  │  │filtered │       │         │selectedCount│     │  │
│  │  │ derived │       │         │  derived    │     │  │
│  │  └────┬────┘       │         └─────┬──────┘     │  │
│  │       │            │               │            │  │
│  │       ▼            ▼               ▼            │  │
│  │  ┌──────────────────────────────────────────┐   │  │
│  │  │              App.svelte                  │   │  │
│  │  │  ┌──────────┐ ┌───────────┐ ┌────────┐  │   │  │
│  │  │  │ProcessTbl│ │StatusBar  │ │ChromeMgr│  │   │  │
│  │  │  └──────────┘ └───────────┘ └────────┘  │   │  │
│  │  │  ┌──────────────────────────────────┐   │   │  │
│  │  │  │   ProcessDetailsModal (on demand)│   │   │  │
│  │  │  └──────────────────────────────────┘   │   │  │
│  │  └──────────────────────────────────────────┘   │  │
│  └─────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## Data Flow

### 1. Polling Cycle

```
onMount() ──▶ startPolling(2000ms)
                  │
                  ▼
          ┌─ fetchMetrics() ◄── setInterval (every 2s)
          │       │
          │       ▼
          │   invoke("get_metrics")  ──▶  Rust get_metrics()
          │       │                              │
          │       │  ◄── JSON { processes, stats }
          │       ▼
          │   applyDiff(current, incoming)
          │       │
          │       ├──▶ processes.set(updated)
          │       ├──▶ stats.set(data.stats)
          │       └──▶ prune dead PIDs from selectedPids
          │
          └─ onDestroy() ──▶ stopPolling() ──▶ clearInterval()
```

### 2. Kill Flow

```
User clicks "Close" ──▶ killSelected()
                            │
                            ▼
                   invoke("kill_processes", { pids })
                            │
                            ▼  Rust: SIGTERM each PID
                   returns killed[]
                            │
                            ▼
                   processes.update(filter out killed)
                   selectedPids.set(empty)
```

### 3. Selection & Filtering

```
search (writable)  ─────────────┐
                                ▼
processes (writable) ──▶ filtered (derived) ──▶ ProcessTable
                                                     │
                                                click row
                                                     │
                                                     ▼
                                          toggleSelect(pid)
                                                     │
                                                     ▼
                                          selectedPids (writable)
                                                     │
                                          ┌──────────┼──────────┐
                                          ▼          ▼          ▼
                                   selectedCount  selectedRamMB  UI
                                    (derived)      (derived)
```

---

## File Map

```
src/
├── main.ts                    App bootstrap (mount to DOM)
├── App.svelte                 Root layout, toolbar, keyboard shortcuts
├── components/
│   ├── ProcessTable.svelte    Sortable table with grouping, row selection
│   ├── ProcessDetailsModal.svelte  Modal with focus trap (Cmd+I / dblclick)
│   ├── ChromeTabManager.svelte     Chrome-specific process section
│   └── StatusBar.svelte       RAM/Swap/Procs gauges
├── stores/
│   └── processes.ts           All state: writable + derived + IPC actions
└── lib/
    └── types.ts               TypeScript interfaces (ProcessEntry, etc.)
```

---

## Store Architecture

All state lives in `stores/processes.ts` as **Svelte stores** (module-level
singletons). Components never call `invoke()` directly — they import store
actions.

| Store | Type | Purpose |
|-------|------|---------|
| `processes` | writable | Full process list from backend |
| `stats` | writable | System-level RAM/swap/count |
| `loading` | writable | Initial load indicator |
| `search` | writable | Filter query string |
| `selectedPids` | writable | Set of user-selected PIDs |
| `focusedPid` | writable | Currently focused row PID |
| `grouping` | writable | Group-by-name toggle |
| `filtered` | derived | Processes matching search query |
| `chromeProcesses` | derived | Chrome-only subset |
| `selectedCount` | derived | Number of selected PIDs |
| `selectedRamMB` | derived | Total RAM of selected PIDs |

### Diff-based refresh

`applyDiff()` compares incoming processes with current state by PID. It reuses
existing object references when metrics haven't changed, preventing unnecessary
Svelte re-renders. Dead PIDs are automatically pruned from both the process
list and the selection set.

---

## IPC Contract (Frontend ↔ Rust)

| Command | Args | Returns | Notes |
|---------|------|---------|-------|
| `get_metrics` | none | `Metrics` | Polled every 2s |
| `kill_process` | `{ pid: u32 }` | `bool` | Single PID |
| `kill_processes` | `{ pids: u32[] }` | `u32[]` | Batch; returns killed PIDs |
| `save_ai_config` | `{ provider, model, key }` | `()` | Keychain storage |
| `apply_ai_rules` | `{ payload: string }` | `number` | Loads versioned AI rules JSON |
| `get_ai_rules_schema` | none | `string` | Returns schema contract JSON |
| `analyze_processes` | `{ profile: string }` | `ProcessSuggestion[]` | AI analysis |

Most IPC is **one-shot** (`invoke()` returns a Promise). The app also consumes
real-time backend events for security alerts.

### Runtime Event Channel

| Event | Payload | Source |
|-------|---------|--------|
| `security-alert` | `DynamicAlert` | Tauri backend thread (`src-tauri/src/lib.rs`) |

The backend deduplicates frequent alerts before emitting, reducing UI churn.

---

## Security Model

### XSS Prevention

Svelte **auto-escapes** all `{expression}` interpolations. The codebase has:
- **Zero** `{@html}` usages
- **Zero** `innerHTML` assignments
- **Zero** `eval()` / `document.write()` calls

Even if a malicious Chrome tab title contains `<script>alert(1)</script>`,
Svelte renders it as safe text: `&lt;script&gt;alert(1)&lt;/script&gt;`.

Style interpolations only use numeric values or hardcoded CSS variable names
from pure functions (`ramColor()`, `cpuColor()`). No backend string ever
reaches a `style=` attribute.

### IPC Hardening

- Frontend → Backend: only typed primitives (`number`, `number[]`, `string`)
- Backend → Frontend: Rust structs serialized via `serde` with strict types
- No user input is ever concatenated into shell commands or SQL

---

## Accessibility (WCAG 2.1 AA)

| Feature | Implementation |
|---------|---------------|
| Modal focus trap | `ProcessDetailsModal` traps Tab/Shift+Tab inside dialog |
| `aria-modal` + `aria-labelledby` | Dialog references `<h2 id="modal-title">` |
| Keyboard navigation | Group headers support Enter/Space; Cmd+F/I/Del shortcuts |
| `aria-sort` on columns | Sortable `<th>` elements announce sort direction |
| `scope="col"` on headers | All table headers have explicit scope |
| `aria-label` on inputs | Search field, checkboxes, and buttons are labeled |
| `aria-live` status bar | Footer announces process count and selection changes |
| `role="progressbar"` | RAM bar has `aria-valuenow/min/max` |
| `aria-expanded` | Collapsible Chrome section and group headers |
| Color contrast | `--fg-dim: #888` passes 4.6:1 on `--bg-alt: #222` |
| Decorative icons | Chevrons and bullets are `aria-hidden="true"` |

---

## Contributing Guidelines

1. **Never call `invoke()` from components.** Add IPC actions to
   `stores/processes.ts` and import them.
2. **Never use `{@html}`** — all backend strings must go through Svelte's
   auto-escaping.
3. **Keep stores flat.** Prefer new `derived` stores over nested state objects.
4. **Test the diff.** After changing store logic, verify that `applyDiff()`
   still reuses object references for unchanged processes.
5. **Check accessibility.** Run `npx vite build` — it reports Svelte a11y
   warnings. The build must pass with zero warnings.
