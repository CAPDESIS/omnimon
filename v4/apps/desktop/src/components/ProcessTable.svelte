<script lang="ts">
  import { onMount } from "svelte";
  import type { ProcessEntry, BrowserTab } from "../lib/types";
  import { toggleSelect, selectedPids, focusedPid, browserTabs } from "../stores/processes";

  interface Props {
    processes: ProcessEntry[];
    grouping?: boolean;
    oninspect?: (proc: ProcessEntry) => void;
  }

  let { processes, grouping = false, oninspect }: Props = $props();

  const ROW_HEIGHT = 20;
  const BUFFER = 10;

  type SortKey = "name" | "pid" | "ram_mb" | "cpu_pct" | "group" | "uptime" | "state";
  let sortKey: SortKey = $state("ram_mb");
  let sortAsc = $state(false);

  // Build a lookup from process name to tab details for the "Detail" column
  let tabDetailMap = $derived.by((): Map<string, BrowserTab> => {
    const map = new Map<string, BrowserTab>();
    for (const tab of $browserTabs) {
      map.set(`Chrome Tab: ${tab.title}`, tab);
    }
    return map;
  });

  function getDetail(proc: ProcessEntry): string {
    const tab = tabDetailMap.get(proc.name);
    if (tab) {
      return `${tab.title} \u2014 ${tab.url}`;
    }
    return proc.exec_name !== proc.name ? proc.exec_name : "";
  }

  function getGroup(proc: ProcessEntry): string {
    if (proc.group === "Browser") {
      const tab = tabDetailMap.get(proc.name);
      if (tab) {
        try {
          return `${tab.browser}: ${new URL(tab.url).hostname}`;
        } catch {
          return tab.browser;
        }
      }
      return "Browser";
    }
    return proc.group || "";
  }

  let collapsedGroups = $state(new Set<string>());

  // Scroll / resize state
  let scrollTop = $state(0);
  let containerHeight = $state(600);
  let wrapEl: HTMLDivElement | undefined = $state();
  let rafId = 0;

  let sorted = $derived(
    [...processes].sort((a, b) => {
      const va = a[sortKey];
      const vb = b[sortKey];
      if (typeof va === "string" && typeof vb === "string") {
        return sortAsc ? va.localeCompare(vb) : vb.localeCompare(va);
      }
      return sortAsc ? Number(va) - Number(vb) : Number(vb) - Number(va);
    }),
  );

  interface ProcessGroup {
    name: string;
    procs: ProcessEntry[];
    totalRam: number;
    totalCpu: number;
    count: number;
  }

  let groups = $derived.by((): ProcessGroup[] => {
    if (!grouping) return [];
    const map = new Map<string, ProcessEntry[]>();
    for (const p of sorted) {
      const arr = map.get(p.name);
      if (arr) arr.push(p);
      else map.set(p.name, [p]);
    }
    return [...map.entries()]
      .map(([name, procs]) => ({
        name,
        procs,
        totalRam: procs.reduce((s, p) => s + p.ram_mb, 0),
        totalCpu: procs.reduce((s, p) => s + p.cpu_pct, 0),
        count: procs.length,
      }))
      .sort((a, b) => b.totalRam - a.totalRam);
  });

  // --- Flat row model for virtual scroll ---
  type FlatRow =
    | { kind: "process"; proc: ProcessEntry }
    | { kind: "group-header"; group: ProcessGroup };

  let flatRows = $derived.by((): FlatRow[] => {
    if (!grouping) {
      return sorted.map((proc) => ({ kind: "process" as const, proc }));
    }
    const rows: FlatRow[] = [];
    for (const group of groups) {
      if (group.count === 1) {
        rows.push({ kind: "process", proc: group.procs[0] });
      } else {
        rows.push({ kind: "group-header", group });
        if (!collapsedGroups.has(group.name)) {
          for (const proc of group.procs) {
            rows.push({ kind: "process", proc });
          }
        }
      }
    }
    return rows;
  });

  // --- Virtual window ---
  let totalHeight = $derived(flatRows.length * ROW_HEIGHT);

  let visibleStartIdx = $derived(
    Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER),
  );
  let visibleEndIdx = $derived(
    Math.min(flatRows.length, Math.ceil((scrollTop + containerHeight) / ROW_HEIGHT) + BUFFER),
  );
  let visibleRows = $derived(flatRows.slice(visibleStartIdx, visibleEndIdx));
  let topSpacerHeight = $derived(visibleStartIdx * ROW_HEIGHT);
  let bottomSpacerHeight = $derived((flatRows.length - visibleEndIdx) * ROW_HEIGHT);

  function onScroll() {
    if (rafId) return;
    rafId = requestAnimationFrame(() => {
      rafId = 0;
      if (wrapEl) scrollTop = wrapEl.scrollTop;
    });
  }

  onMount(() => {
    if (!wrapEl) return;
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        containerHeight = entry.contentRect.height;
      }
    });
    ro.observe(wrapEl);
    containerHeight = wrapEl.clientHeight;
    return () => {
      ro.disconnect();
      if (rafId) cancelAnimationFrame(rafId);
    };
  });

  function setSort(key: SortKey) {
    if (sortKey === key) sortAsc = !sortAsc;
    else {
      sortKey = key;
      sortAsc = key === "name" || key === "group";
    }
  }

  function arrow(key: SortKey): string {
    if (sortKey !== key) return "";
    return sortAsc ? " \u25B2" : " \u25BC";
  }

  function ramColor(mb: number): string {
    if (mb >= 1024) return "var(--danger)";
    if (mb >= 256) return "var(--yellow)";
    return "var(--fg)";
  }

  function cpuColor(pct: number): string {
    if (pct >= 50) return "var(--danger)";
    if (pct >= 10) return "var(--yellow)";
    return "var(--fg)";
  }

  function handleRowClick(proc: ProcessEntry) {
    toggleSelect(proc.pid);
    $focusedPid = proc.pid;
  }

  function handleRowDblClick(proc: ProcessEntry) {
    oninspect?.(proc);
  }

  function toggleCollapse(name: string) {
    const next = new Set(collapsedGroups);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    collapsedGroups = next;
  }
</script>

{#snippet processRow(proc: ProcessEntry)}
  <tr
    class:selected={$selectedPids.has(proc.pid)}
    class:system={proc.is_system}
    class:focused={$focusedPid === proc.pid}
    onclick={() => handleRowClick(proc)}
    ondblclick={() => handleRowDblClick(proc)}
  >
    <td class="col-check">
      <input
        type="checkbox"
        checked={$selectedPids.has(proc.pid)}
        disabled={proc.is_system}
        aria-label="Select {proc.name}"
        onclick={(e: MouseEvent) => { e.stopPropagation(); toggleSelect(proc.pid); }}
      />
    </td>
    <td class="col-name" title={proc.exec_name}>
      <span class="name-text">{proc.name}</span>
      {#if proc.idle}<span class="badge idle">idle</span>{/if}
    </td>
    <td class="col-detail" title={getDetail(proc)}>
      <span class="detail-text">{getDetail(proc)}</span>
    </td>
    <td class="col-group" title={getGroup(proc)}>
      <span class="group-text">{getGroup(proc)}</span>
    </td>
    <td class="col-ram mono" style="color: {ramColor(proc.ram_mb)}">
      {proc.ram_mb.toFixed(1)}
    </td>
    <td class="col-cpu mono" style="color: {cpuColor(proc.cpu_pct)}">
      {proc.cpu_pct.toFixed(1)}
    </td>
    <td class="col-uptime mono">{proc.uptime || "\u2014"}</td>
    <td class="col-pid mono">{proc.pid}</td>
    <td class="col-state mono">{proc.state}</td>
  </tr>
{/snippet}

{#snippet groupHeaderRow(group: ProcessGroup)}
  <tr
    class="group-header"
    onclick={() => toggleCollapse(group.name)}
    onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggleCollapse(group.name); } }}
    tabindex="0"
    role="button"
    aria-expanded={!collapsedGroups.has(group.name)}
  >
    <td class="col-check"></td>
    <td colspan="8" class="group-cell">
      <span class="chevron" class:open={!collapsedGroups.has(group.name)} aria-hidden="true">&#9654;</span>
      <span class="group-name">{group.name}</span>
      <span class="group-meta">
        {group.count} &middot; {group.totalRam.toFixed(0)} MB &middot; {group.totalCpu.toFixed(1)}%
      </span>
    </td>
  </tr>
{/snippet}

<div class="table-wrap" bind:this={wrapEl} onscroll={onScroll}>
  <table aria-label="Process list">
    <thead>
      <tr>
        <th class="col-check" scope="col"><span class="sr-only">Select</span></th>
        <th class="col-name sortable" scope="col" aria-sort={sortKey === "name" ? (sortAsc ? "ascending" : "descending") : "none"} onclick={() => setSort("name")}>
          Name<span aria-hidden="true">{arrow("name")}</span>
        </th>
        <th class="col-detail" scope="col">Detail</th>
        <th class="col-group sortable" scope="col" aria-sort={sortKey === "group" ? (sortAsc ? "ascending" : "descending") : "none"} onclick={() => setSort("group")}>
          Group<span aria-hidden="true">{arrow("group")}</span>
        </th>
        <th class="col-ram sortable" scope="col" aria-sort={sortKey === "ram_mb" ? (sortAsc ? "ascending" : "descending") : "none"} onclick={() => setSort("ram_mb")}>
          RAM<span aria-hidden="true">{arrow("ram_mb")}</span>
        </th>
        <th class="col-cpu sortable" scope="col" aria-sort={sortKey === "cpu_pct" ? (sortAsc ? "ascending" : "descending") : "none"} onclick={() => setSort("cpu_pct")}>
          CPU<span aria-hidden="true">{arrow("cpu_pct")}</span>
        </th>
        <th class="col-uptime sortable" scope="col" aria-sort={sortKey === "uptime" ? (sortAsc ? "ascending" : "descending") : "none"} onclick={() => setSort("uptime")}>
          Time<span aria-hidden="true">{arrow("uptime")}</span>
        </th>
        <th class="col-pid sortable" scope="col" aria-sort={sortKey === "pid" ? (sortAsc ? "ascending" : "descending") : "none"} onclick={() => setSort("pid")}>
          PID<span aria-hidden="true">{arrow("pid")}</span>
        </th>
        <th class="col-state sortable" scope="col" aria-sort={sortKey === "state" ? (sortAsc ? "ascending" : "descending") : "none"} onclick={() => setSort("state")}>
          ST<span aria-hidden="true">{arrow("state")}</span>
        </th>
      </tr>
    </thead>
    <tbody>
      {#if topSpacerHeight > 0}
        <tr class="spacer" aria-hidden="true"><td style="height:{topSpacerHeight}px" colspan="9"></td></tr>
      {/if}
      {#each visibleRows as row, i (row.kind === "process" ? `p-${row.proc.pid}` : `g-${row.group.name}`)}
        {#if row.kind === "process"}
          {@render processRow(row.proc)}
        {:else}
          {@render groupHeaderRow(row.group)}
        {/if}
      {/each}
      {#if bottomSpacerHeight > 0}
        <tr class="spacer" aria-hidden="true"><td style="height:{bottomSpacerHeight}px" colspan="9"></td></tr>
      {/if}
    </tbody>
  </table>
</div>

<style>
  .table-wrap {
    flex: 1;
    overflow-y: auto;
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
    table-layout: fixed;
  }

  thead {
    position: sticky;
    top: 0;
    z-index: 2;
  }

  th {
    height: 20px;
    padding: 0 6px;
    text-align: left;
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border);
    color: var(--fg-dim);
    font-weight: 600;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    user-select: none;
    white-space: nowrap;
    line-height: 20px;
  }
  th.sortable {
    cursor: pointer;
  }
  th.sortable:hover {
    color: var(--fg);
  }

  td {
    height: 20px;
    padding: 0 6px;
    border-bottom: 1px solid var(--border-subtle, rgba(128, 128, 128, 0.15));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 20px;
  }

  tr {
    cursor: default;
  }
  tr:hover:not(.spacer) {
    background: var(--bg-hover);
  }
  tr.selected {
    background: var(--bg-selected);
  }
  tr.focused {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }
  tr.system {
    opacity: 0.45;
  }
  tr.system:hover {
    opacity: 0.65;
  }
  tr.spacer td {
    padding: 0;
    border: none;
  }

  .group-header {
    cursor: pointer;
    background: var(--bg-alt);
  }
  .group-header:hover {
    background: var(--bg-hover);
  }
  .group-cell {
    font-weight: 600;
    font-size: 11px;
    padding: 0 6px;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 20px;
  }

  .chevron {
    font-size: 8px;
    color: var(--fg-dim);
    transition: transform 0.15s ease;
    display: inline-block;
  }
  .chevron.open {
    transform: rotate(90deg);
  }

  .group-name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .group-meta {
    color: var(--fg-dim);
    font-weight: 400;
    font-size: 10px;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    flex-shrink: 0;
  }

  .mono {
    font-variant-numeric: tabular-nums;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 10.5px;
  }

  .col-check {
    width: 24px;
    text-align: center;
  }
  .col-name {
    width: 16%;
    min-width: 100px;
  }
  .col-detail {
    width: 24%;
    min-width: 140px;
    color: var(--fg-dim);
    font-size: 10px;
  }
  .col-group {
    width: 12%;
    min-width: 80px;
    color: var(--fg-dim);
    font-size: 10px;
  }
  .col-ram {
    width: 55px;
    text-align: right;
  }
  .col-cpu {
    width: 48px;
    text-align: right;
  }
  .col-uptime {
    width: 48px;
    text-align: right;
    color: var(--fg-dim);
  }
  .col-pid {
    width: 55px;
    text-align: right;
  }
  .col-state {
    width: 24px;
    text-align: center;
    color: var(--fg-dim);
  }

  .detail-text,
  .group-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
  }

  input[type="checkbox"] {
    margin: 0;
    cursor: pointer;
    width: 12px;
    height: 12px;
  }

  .name-text {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .badge {
    display: inline-block;
    padding: 0 3px;
    border-radius: 2px;
    font-size: 8px;
    font-weight: 700;
    margin-left: 4px;
    vertical-align: middle;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .badge.idle {
    background: rgba(255, 193, 7, 0.15);
    color: var(--yellow);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
