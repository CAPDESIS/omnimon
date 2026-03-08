<script lang="ts">
  import { onMount } from "svelte";
  import type { ProcessEntry } from "../lib/types";
  import type { ColumnConfig, ColumnKey } from "../stores/preferences";
  import { COLUMN_KEYS } from "../stores/preferences";
  import { toggleSelect, selectedPids, focusedPid, browserTabs } from "../stores/processes";
  import { fontSize, moveColumnToIndex } from "../stores/preferences";
  import { t } from "../lib/i18n";
  import { detectBrowser } from "../lib/browser";
  import SecurityBadge from "./SecurityBadge.svelte";
  import { iconForProcess, isNativeIconDataUrl } from "../lib/processIcons";
  import {
    PROCESS_TABLE_ROW_BUFFER,
    RAM_THRESHOLD_DANGER,
    RAM_THRESHOLD_WARNING,
    CPU_THRESHOLD_DANGER,
    CPU_THRESHOLD_WARNING,
    ENERGY_THRESHOLD_DANGER,
    ENERGY_THRESHOLD_WARNING,
    BYTES_PER_MB,
    BYTES_PER_KB,
  } from "../lib/constants";

  interface Props {
    processes: ProcessEntry[];
    grouping?: boolean;
    columns?: ColumnConfig;
    columnOrder?: ColumnKey[];
    oninspect?: (proc: ProcessEntry) => void;
  }

  let { processes, grouping = false, columns, columnOrder, oninspect }: Props = $props();

  let cols = $derived(columns ?? { name: true, detail: true, group: true, ram: true, cpu: true, energy: true, network: true, uptime: true, pid: true, state: true });
  let orderedVisibleCols = $derived(
    (columnOrder ?? COLUMN_KEYS).filter((k): k is ColumnKey => cols[k]),
  );
  let visibleColCount = $derived(orderedVisibleCols.length);

  let ROW_HEIGHT = $derived(Math.round($fontSize * 1.667));
  const BUFFER = PROCESS_TABLE_ROW_BUFFER;

  type SortKey = "name" | "pid" | "ram_mb" | "cpu_pct" | "energy_metric" | "network_metric" | "group" | "uptime" | "state";
  let sortKey: SortKey = $state("ram_mb");
  let sortAsc = $state(false);

  function energyMetric(proc: ProcessEntry): number {
    return proc.energy_impact_score ?? 0;
  }

  function networkMetric(proc: ProcessEntry): number {
    return proc.net_rx_bytes_per_sec + proc.net_tx_bytes_per_sec;
  }

  function sortValue(proc: ProcessEntry, key: SortKey): string | number {
    if (key === "energy_metric") return energyMetric(proc);
    if (key === "network_metric") return networkMetric(proc);
    if (key === "group") return getGroup(proc);
    return proc[key];
  }

  // Count tabs per browser for the "Detail" column
  let tabCountByBrowser = $derived.by((): Map<string, number> => {
    const counts = new Map<string, number>();
    for (const tab of $browserTabs) {
      counts.set(tab.browser, (counts.get(tab.browser) ?? 0) + 1);
    }
    return counts;
  });

  function getDetail(proc: ProcessEntry): string {
    const browser = detectBrowser(proc);
    if (browser) {
      const count = tabCountByBrowser.get(browser);
      if (count) return count !== 1
        ? t("table.tabsOpenPlural", { count, browser })
        : t("table.tabsOpen", { count, browser });
    }
    return proc.exec_name !== proc.name ? proc.exec_name : "";
  }

  function getGroup(proc: ProcessEntry): string {
    const browser = detectBrowser(proc);
    if (browser) return browser;
    return proc.group || proc.grouped_name || "";
  }

  let collapsedGroups = $state(new Set<string>());

  // Scroll / resize state
  let scrollTop = $state(0);
  let containerHeight = $state(600);
  let wrapEl: HTMLDivElement | undefined = $state();
  let rafId = 0;
  let draggedColumn = $state<ColumnKey | null>(null);

  let processByPid = $derived.by((): Map<number, ProcessEntry> => {
    const map = new Map<number, ProcessEntry>();
    for (const proc of processes) map.set(proc.pid, proc);
    return map;
  });

  let _sortedSnapshot = "";
  let _sortedKey: SortKey = "ram_mb";
  let _sortedAsc = false;
  let _sortedPidsCache: number[] = [];

  let sortedPids = $derived.by((): number[] => {
    let snapshot = `${sortKey}:${sortAsc ? 1 : 0}:${processes.length}|`;
    for (const proc of processes) {
      snapshot += `${proc.pid}:${String(sortValue(proc, sortKey))}|`;
    }

    if (snapshot === _sortedSnapshot && sortKey === _sortedKey && sortAsc === _sortedAsc) {
      return _sortedPidsCache;
    }

    const sorted = [...processes].sort((a, b) => {
      const va = sortValue(a, sortKey);
      const vb = sortValue(b, sortKey);
      if (typeof va === "string" && typeof vb === "string") {
        return sortAsc ? va.localeCompare(vb) : vb.localeCompare(va);
      }
      return sortAsc ? Number(va) - Number(vb) : Number(vb) - Number(va);
    });

    _sortedSnapshot = snapshot;
    _sortedKey = sortKey;
    _sortedAsc = sortAsc;
    _sortedPidsCache = sorted.map((proc) => proc.pid);
    return _sortedPidsCache;
  });

  // --- Rank change tracking for micro-animations ---
  // Use plain variables (not $state) to avoid effect cycles
  let _prevRanks = new Map<number, number>();
  let movedUpPids = $state(new Set<number>());

  $effect(() => {
    const items = sortedPids; // subscribe to sorted rank changes
    const newRanks = new Map<number, number>();
    for (let i = 0; i < items.length; i++) {
      newRanks.set(items[i], i);
    }

    const moved = new Set<number>();
    for (const [pid, newIdx] of newRanks) {
      const oldIdx = _prevRanks.get(pid);
      if (oldIdx !== undefined && oldIdx > newIdx && oldIdx - newIdx >= 2) {
        moved.add(pid);
      }
    }

    _prevRanks = newRanks;

    if (moved.size > 0) {
      movedUpPids = moved;
      setTimeout(() => { movedUpPids = new Set(); }, 600);
    }
  });

  interface ProcessGroup {
    key: string;
    name: string;
    procs: ProcessEntry[];
    totalRam: number;
    totalCpu: number;
    count: number;
    totalNetwork: number;
    totalEnergy: number;
  }

  function getGroupIdentity(proc: ProcessEntry): { key: string; label: string } {
    const browser = detectBrowser(proc);
    const label = (proc.grouped_name || browser || proc.name || proc.exec_name || proc.group || "Unknown").trim();
    const key = (proc.group_key || `${label}:${proc.exec_name}:${proc.group_identity_type || proc.group || "proc"}`).trim();
    return { key, label };
  }

  let groups = $derived.by((): ProcessGroup[] => {
    if (!grouping) return [];
    const map = new Map<string, { label: string; procs: ProcessEntry[] }>();
    for (const pid of sortedPids) {
      const p = processByPid.get(pid);
      if (!p) continue;
      const identity = getGroupIdentity(p);
      const group = map.get(identity.key);
      if (group) group.procs.push(p);
      else map.set(identity.key, { label: identity.label, procs: [p] });
    }
    return [...map.entries()]
      .map(([key, value]) => ({
        key,
        name: value.label,
        procs: value.procs,
        totalRam: value.procs.reduce((s, p) => s + p.ram_mb, 0),
        totalCpu: value.procs.reduce((s, p) => s + p.cpu_pct, 0),
        count: value.procs.length,
        totalNetwork: value.procs.reduce((s, p) => s + networkMetric(p), 0),
        totalEnergy: value.procs.reduce((s, p) => s + energyMetric(p), 0),
      }))
      .sort((a, b) => b.totalRam - a.totalRam);
  });

  // --- Flat row model for virtual scroll ---
  type FlatRow =
    | { kind: "process"; pid: number }
    | { kind: "group-header"; group: ProcessGroup };

  let flatRows = $derived.by((): FlatRow[] => {
    if (!grouping) {
      return sortedPids.map((pid) => ({ kind: "process" as const, pid }));
    }
    const rows: FlatRow[] = [];
    for (const group of groups) {
      if (group.count === 1) {
        rows.push({ kind: "process", pid: group.procs[0].pid });
      } else {
        rows.push({ kind: "group-header", group });
        if (!collapsedGroups.has(group.key)) {
          for (const proc of group.procs) {
            rows.push({ kind: "process", pid: proc.pid });
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
    if (mb >= RAM_THRESHOLD_DANGER) return "var(--danger)";
    if (mb >= RAM_THRESHOLD_WARNING) return "var(--yellow)";
    return "var(--fg)";
  }

  function cpuColor(pct: number): string {
    if (pct >= CPU_THRESHOLD_DANGER) return "var(--danger)";
    if (pct >= CPU_THRESHOLD_WARNING) return "var(--yellow)";
    return "var(--fg)";
  }

  function energyColor(score: number | null): string {
    const value = score ?? 0;
    if (value >= ENERGY_THRESHOLD_DANGER) return "var(--danger)";
    if (value >= ENERGY_THRESHOLD_WARNING) return "var(--yellow)";
    return "var(--fg)";
  }

  function formatNetworkRate(bytesPerSec: number): string {
    if (bytesPerSec >= BYTES_PER_MB) return `${(bytesPerSec / BYTES_PER_MB).toFixed(1)} MB/s`;
    if (bytesPerSec >= BYTES_PER_KB) return `${(bytesPerSec / BYTES_PER_KB).toFixed(1)} KB/s`;
    return `${bytesPerSec} B/s`;
  }

  function groupRowNetworkRate(group: ProcessGroup): string {
    return formatNetworkRate(group.totalNetwork);
  }

  function groupRowEnergy(group: ProcessGroup): string {
    return group.totalEnergy.toFixed(1);
  }

  function handleRowClick(proc: ProcessEntry) {
    toggleSelect(proc.pid);
    $focusedPid = proc.pid;
  }

  function handleRowDblClick(proc: ProcessEntry) {
    oninspect?.(proc);
  }

  function handleRowKeydown(event: KeyboardEvent, proc: ProcessEntry) {
    if (event.key === "Enter") {
      event.preventDefault();
      oninspect?.(proc);
      return;
    }
    if (event.key === " ") {
      event.preventDefault();
      handleRowClick(proc);
    }
  }

  function sortLabel(key: SortKey): string {
    const direction = sortKey === key ? (sortAsc ? "ascending" : "descending") : "sortable";
    return `${key} ${direction}`;
  }

  function toggleCollapse(key: string) {
    const next = new Set(collapsedGroups);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    collapsedGroups = next;
  }

  function handleColumnDragStart(key: ColumnKey) {
    draggedColumn = key;
  }

  function handleColumnDrop(targetKey: ColumnKey) {
    if (!draggedColumn || draggedColumn === targetKey) return;
    const targetIndex = orderedVisibleCols.indexOf(targetKey);
    if (targetIndex >= 0) moveColumnToIndex(draggedColumn, targetIndex);
    draggedColumn = null;
  }

  function allowColumnDrop(event: DragEvent) {
    event.preventDefault();
  }
</script>

{#snippet colCell(key: ColumnKey, proc: ProcessEntry)}
  {#if key === "name"}
    <td class="col-name" title={proc.exec_name}>
      {#if isNativeIconDataUrl(proc.icon_data_url)}
        <img class="proc-icon native" src={proc.icon_data_url} alt="" aria-hidden="true" />
      {:else}
        <svg class="proc-icon" viewBox="0 0 16 16" width="12" height="12" fill="currentColor" aria-hidden="true">
          <path d={iconForProcess(proc.name, proc.group)} />
        </svg>
      {/if}
      <span class="name-text">{proc.name}</span>
      {#if !grouping && proc.process_count > 1}<span class="badge grouped">x{proc.process_count}</span>{/if}
      {#if proc.idle}<span class="badge idle">{t("table.idle")}</span>{/if}
      <SecurityBadge pid={proc.pid} />
    </td>
  {:else if key === "detail"}
    {@const detail = getDetail(proc)}
    <td class="col-detail" title={detail}>
      <span class="detail-text">{detail}</span>
    </td>
  {:else if key === "group"}
    {@const group = getGroup(proc)}
    <td class="col-group" title={group}>
      <span class="group-text">{group}</span>
    </td>
  {:else if key === "ram"}
    <td class="col-ram mono" style="color: {ramColor(proc.ram_mb)}">{proc.ram_mb.toFixed(1)}</td>
  {:else if key === "cpu"}
    <td class="col-cpu mono" style="color: {cpuColor(proc.cpu_pct)}">{proc.cpu_pct.toFixed(1)}</td>
  {:else if key === "energy"}
    <td class="col-energy mono" style="color: {energyColor(proc.energy_impact_score)}">{(proc.energy_impact_score ?? 0).toFixed(1)}</td>
  {:else if key === "network"}
    <td class="col-network mono" title={formatNetworkRate(networkMetric(proc))}>{formatNetworkRate(networkMetric(proc))}</td>
  {:else if key === "uptime"}
    <td class="col-uptime mono">{proc.uptime || "\u2014"}</td>
  {:else if key === "pid"}
    <td class="col-pid mono">{proc.pid}</td>
  {:else if key === "state"}
    <td class="col-state mono">{proc.state}</td>
  {/if}
{/snippet}

{#snippet processRow(proc: ProcessEntry)}
  <tr
    class:selected={$selectedPids.has(proc.pid)}
    class:system={proc.is_system}
    class:focused={$focusedPid === proc.pid}
    class:rank-up={movedUpPids.has(proc.pid)}
    onclick={() => handleRowClick(proc)}
    ondblclick={() => handleRowDblClick(proc)}
    onkeydown={(event: KeyboardEvent) => handleRowKeydown(event, proc)}
    tabindex={proc.is_system ? -1 : 0}
  >
    <td class="col-check">
      <input
        type="checkbox"
        checked={$selectedPids.has(proc.pid)}
        disabled={proc.is_system}
        aria-label={t("table.selectProcess", { name: proc.name })}
        onclick={(e: MouseEvent) => { e.stopPropagation(); toggleSelect(proc.pid); }}
      />
    </td>
    {#each orderedVisibleCols as key (key)}
      {@render colCell(key, proc)}
    {/each}
  </tr>
{/snippet}

{#snippet groupHeaderRow(group: ProcessGroup)}
  <tr class="group-header">
    <td class="col-check"></td>
    <td colspan={visibleColCount} class="group-cell">
      <button type="button" class="group-toggle" onclick={() => toggleCollapse(group.key)} aria-expanded={!collapsedGroups.has(group.key)} aria-label={t("table.toggleGroup", { name: group.name })}>
        <span class="chevron" class:open={!collapsedGroups.has(group.key)} aria-hidden="true">&#9654;</span>
        <span class="group-name">{group.name}</span>
        <span class="badge grouped">x{group.count}</span>
        <span class="group-meta">
          {group.count} &middot; {group.totalRam.toFixed(0)} MB &middot; {group.totalCpu.toFixed(1)}% &middot; {groupRowEnergy(group)} E &middot; {groupRowNetworkRate(group)}
        </span>
      </button>
    </td>
  </tr>
{/snippet}

<div class="table-wrap" bind:this={wrapEl} onscroll={onScroll}>
  <table aria-label={t("table.processList")}>
    <thead>
      <tr>
        <th class="col-check" scope="col"><span class="sr-only">{t("table.select")}</span></th>
        {#each orderedVisibleCols as key (key)}
          {#if key === "name"}
            <th class="col-name sortable" draggable="true" ondragstart={() => handleColumnDragStart("name")} ondragover={allowColumnDrop} ondrop={() => handleColumnDrop("name")} scope="col" aria-sort={sortKey === "name" ? (sortAsc ? "ascending" : "descending") : "none"}>
              <button type="button" class="sort-button" aria-label={sortLabel("name")} onclick={() => setSort("name")}>{t("table.name")}<span aria-hidden="true">{arrow("name")}</span></button>
            </th>
          {:else if key === "detail"}
            <th class="col-detail" draggable="true" ondragstart={() => handleColumnDragStart("detail")} ondragover={allowColumnDrop} ondrop={() => handleColumnDrop("detail")} scope="col">{t("table.detail")}</th>
          {:else if key === "group"}
            <th class="col-group sortable" draggable="true" ondragstart={() => handleColumnDragStart("group")} ondragover={allowColumnDrop} ondrop={() => handleColumnDrop("group")} scope="col" aria-sort={sortKey === "group" ? (sortAsc ? "ascending" : "descending") : "none"}>
              <button type="button" class="sort-button" aria-label={sortLabel("group")} onclick={() => setSort("group")}>{t("table.group")}<span aria-hidden="true">{arrow("group")}</span></button>
            </th>
          {:else if key === "ram"}
            <th class="col-ram sortable" draggable="true" ondragstart={() => handleColumnDragStart("ram")} ondragover={allowColumnDrop} ondrop={() => handleColumnDrop("ram")} scope="col" aria-sort={sortKey === "ram_mb" ? (sortAsc ? "ascending" : "descending") : "none"}>
              <button type="button" class="sort-button sort-button-num" aria-label={sortLabel("ram_mb")} onclick={() => setSort("ram_mb")}>{t("table.ram")}<span aria-hidden="true">{arrow("ram_mb")}</span></button>
            </th>
          {:else if key === "cpu"}
            <th class="col-cpu sortable" draggable="true" ondragstart={() => handleColumnDragStart("cpu")} ondragover={allowColumnDrop} ondrop={() => handleColumnDrop("cpu")} scope="col" aria-sort={sortKey === "cpu_pct" ? (sortAsc ? "ascending" : "descending") : "none"}>
              <button type="button" class="sort-button sort-button-num" aria-label={sortLabel("cpu_pct")} onclick={() => setSort("cpu_pct")}>{t("table.cpu")}<span aria-hidden="true">{arrow("cpu_pct")}</span></button>
            </th>
          {:else if key === "uptime"}
            <th class="col-uptime sortable" draggable="true" ondragstart={() => handleColumnDragStart("uptime")} ondragover={allowColumnDrop} ondrop={() => handleColumnDrop("uptime")} scope="col" aria-sort={sortKey === "uptime" ? (sortAsc ? "ascending" : "descending") : "none"}>
              <button type="button" class="sort-button sort-button-num" aria-label={sortLabel("uptime")} onclick={() => setSort("uptime")}>{t("table.time")}<span aria-hidden="true">{arrow("uptime")}</span></button>
            </th>
          {:else if key === "energy"}
            <th class="col-energy sortable" draggable="true" ondragstart={() => handleColumnDragStart("energy")} ondragover={allowColumnDrop} ondrop={() => handleColumnDrop("energy")} scope="col" aria-sort={sortKey === "energy_metric" ? (sortAsc ? "ascending" : "descending") : "none"}>
              <button type="button" class="sort-button sort-button-num" aria-label={sortLabel("energy_metric")} onclick={() => setSort("energy_metric")}>{t("table.energy")}<span aria-hidden="true">{arrow("energy_metric")}</span></button>
            </th>
          {:else if key === "network"}
            <th class="col-network sortable" draggable="true" ondragstart={() => handleColumnDragStart("network")} ondragover={allowColumnDrop} ondrop={() => handleColumnDrop("network")} scope="col" aria-sort={sortKey === "network_metric" ? (sortAsc ? "ascending" : "descending") : "none"}>
              <button type="button" class="sort-button sort-button-num" aria-label={sortLabel("network_metric")} onclick={() => setSort("network_metric")}>{t("table.network")}<span aria-hidden="true">{arrow("network_metric")}</span></button>
            </th>
          {:else if key === "pid"}
            <th class="col-pid sortable" draggable="true" ondragstart={() => handleColumnDragStart("pid")} ondragover={allowColumnDrop} ondrop={() => handleColumnDrop("pid")} scope="col" aria-sort={sortKey === "pid" ? (sortAsc ? "ascending" : "descending") : "none"}>
              <button type="button" class="sort-button sort-button-num" aria-label={sortLabel("pid")} onclick={() => setSort("pid")}>{t("table.pid")}<span aria-hidden="true">{arrow("pid")}</span></button>
            </th>
          {:else if key === "state"}
            <th class="col-state sortable" draggable="true" ondragstart={() => handleColumnDragStart("state")} ondragover={allowColumnDrop} ondrop={() => handleColumnDrop("state")} scope="col" aria-sort={sortKey === "state" ? (sortAsc ? "ascending" : "descending") : "none"}>
              <button type="button" class="sort-button" aria-label={sortLabel("state")} onclick={() => setSort("state")}>{t("table.st")}<span aria-hidden="true">{arrow("state")}</span></button>
            </th>
          {/if}
        {/each}
      </tr>
    </thead>
    <tbody>
      {#if topSpacerHeight > 0}
        <tr class="spacer" aria-hidden="true"><td style="height:{topSpacerHeight}px" colspan={visibleColCount + 1}></td></tr>
      {/if}
      {#each visibleRows as row (row.kind === "process" ? `p-${row.pid}` : `g-${row.group.key}`)}
        {#if row.kind === "process"}
          {@const proc = processByPid.get(row.pid)}
          {#if proc}
            {@render processRow(proc)}
          {/if}
        {:else}
          {@render groupHeaderRow(row.group)}
        {/if}
      {/each}
      {#if bottomSpacerHeight > 0}
        <tr class="spacer" aria-hidden="true"><td style="height:{bottomSpacerHeight}px" colspan={visibleColCount + 1}></td></tr>
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
    font-size: calc(var(--base-font-size) * 0.917);
    table-layout: fixed;
  }

  thead {
    position: sticky;
    top: 0;
    z-index: 2;
  }

  th {
    height: calc(var(--base-font-size) * 1.667);
    padding: 0 6px;
    text-align: left;
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border);
    color: var(--fg-dim);
    font-weight: 600;
    font-size: calc(var(--base-font-size) * 0.833);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    user-select: none;
    white-space: nowrap;
    line-height: calc(var(--base-font-size) * 1.667);
  }
  th.sortable {
    cursor: default;
  }

  .sort-button {
    width: 100%;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    text-transform: inherit;
    letter-spacing: inherit;
    text-align: left;
    padding: 0;
    cursor: pointer;
  }

  .sort-button-num {
    text-align: right;
  }

  .sort-button:hover,
  .sort-button:focus-visible {
    color: var(--fg);
  }

  td {
    height: calc(var(--base-font-size) * 1.667);
    padding: 0 6px;
    border-bottom: 1px solid var(--border-subtle, rgba(128, 128, 128, 0.15));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: calc(var(--base-font-size) * 1.667);
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
    background: var(--bg-alt);
  }
  .group-header:hover {
    background: var(--bg-hover);
  }
  .group-cell {
    padding: 0;
  }

  .group-toggle {
    width: 100%;
    height: calc(var(--base-font-size) * 1.667);
    border: none;
    background: transparent;
    font-weight: 600;
    font-size: calc(var(--base-font-size) * 0.917);
    color: var(--fg);
    padding: 0 6px;
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }

  .group-toggle:hover,
  .group-toggle:focus-visible {
    background: var(--bg-hover);
  }

  .chevron {
    font-size: calc(var(--base-font-size) * 0.667);
    color: var(--fg-dim);
    transition: transform 0.15s ease;
    display: inline-block;
  }
  .chevron.open {
    transform: rotate(90deg);
  }

  .group-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .group-meta {
    color: var(--fg-dim);
    font-weight: 400;
    font-size: calc(var(--base-font-size) * 0.833);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    flex-shrink: 0;
    max-width: 48%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mono {
    font-variant-numeric: tabular-nums;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size) * 0.875);
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
    font-size: calc(var(--base-font-size) * 0.833);
  }
  .col-group {
    width: 12%;
    min-width: 80px;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 0.833);
  }
  .col-ram {
    width: 90px;
    text-align: right;
  }
  .col-cpu {
    width: 76px;
    text-align: right;
  }
  .col-uptime {
    width: 72px;
    text-align: right;
    color: var(--fg-dim);
  }
  .col-pid {
    width: 72px;
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

  th[draggable="true"] {
    position: relative;
  }

  th[draggable="true"]::after {
    content: "⋮⋮";
    margin-left: 6px;
    color: var(--fg-dim);
    font-size: 10px;
    letter-spacing: -1px;
  }

  input[type="checkbox"] {
    margin: 0;
    cursor: pointer;
    width: 12px;
    height: 12px;
  }

  .proc-icon {
    flex-shrink: 0;
    color: var(--fg-dim);
    vertical-align: middle;
    margin-right: 3px;
  }

  .proc-icon.native {
    width: 12px;
    height: 12px;
    object-fit: contain;
    border-radius: 2px;
  }

  .name-text {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  tr.rank-up {
    animation: rank-pulse 0.5s ease-out;
  }

  @keyframes rank-pulse {
    0% { background: var(--accent-dim, rgba(59,130,246,0.15)); }
    100% { background: transparent; }
  }

  .badge {
    display: inline-block;
    padding: 0 3px;
    border-radius: 2px;
    font-size: calc(var(--base-font-size) * 0.667);
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

  .badge.grouped {
    background: rgba(59, 130, 246, 0.14);
    color: var(--accent);
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
