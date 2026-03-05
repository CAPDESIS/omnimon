<script lang="ts">
  import type { ProcessEntry } from "../lib/types";
  import { toggleSelect, selectedPids, focusedPid } from "../stores/processes";

  interface Props {
    processes: ProcessEntry[];
    grouping?: boolean;
    oninspect?: (proc: ProcessEntry) => void;
  }

  let { processes, grouping = false, oninspect }: Props = $props();

  type SortKey = "name" | "pid" | "ram_mb" | "cpu_pct" | "group";
  let sortKey: SortKey = $state("ram_mb");
  let sortAsc = $state(false);

  let collapsedGroups = $state(new Set<string>());

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
    <td class="col-pid mono">{proc.pid}</td>
    <td class="col-ram mono" style="color: {ramColor(proc.ram_mb)}">
      {proc.ram_mb.toFixed(1)}
    </td>
    <td class="col-cpu mono" style="color: {cpuColor(proc.cpu_pct)}">
      {proc.cpu_pct.toFixed(1)}
    </td>
    <td class="col-uptime mono">{proc.uptime || "—"}</td>
    <td class="col-state mono">{proc.state}</td>
  </tr>
{/snippet}

<div class="table-wrap">
  <table aria-label="Process list">
    <thead>
      <tr>
        <th class="col-check" scope="col"><span class="sr-only">Select</span></th>
        <th class="col-name sortable" scope="col" aria-sort={sortKey === "name" ? (sortAsc ? "ascending" : "descending") : "none"} onclick={() => setSort("name")}>
          Name<span aria-hidden="true">{arrow("name")}</span>
        </th>
        <th class="col-pid sortable" scope="col" aria-sort={sortKey === "pid" ? (sortAsc ? "ascending" : "descending") : "none"} onclick={() => setSort("pid")}>
          PID<span aria-hidden="true">{arrow("pid")}</span>
        </th>
        <th class="col-ram sortable" scope="col" aria-sort={sortKey === "ram_mb" ? (sortAsc ? "ascending" : "descending") : "none"} onclick={() => setSort("ram_mb")}>
          RAM<span aria-hidden="true">{arrow("ram_mb")}</span>
        </th>
        <th class="col-cpu sortable" scope="col" aria-sort={sortKey === "cpu_pct" ? (sortAsc ? "ascending" : "descending") : "none"} onclick={() => setSort("cpu_pct")}>
          CPU<span aria-hidden="true">{arrow("cpu_pct")}</span>
        </th>
        <th class="col-uptime" scope="col">Up</th>
        <th class="col-state" scope="col">St</th>
      </tr>
    </thead>
    <tbody>
      {#if grouping}
        {#each groups as group (group.name)}
          {#if group.count === 1}
            {@render processRow(group.procs[0])}
          {:else}
            <tr
              class="group-header"
              onclick={() => toggleCollapse(group.name)}
              onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggleCollapse(group.name); } }}
              tabindex="0"
              role="button"
              aria-expanded={!collapsedGroups.has(group.name)}
            >
              <td class="col-check"></td>
              <td colspan="6" class="group-cell">
                <span class="chevron" class:open={!collapsedGroups.has(group.name)} aria-hidden="true">&#9654;</span>
                <span class="group-name">{group.name}</span>
                <span class="group-meta">
                  {group.count} &middot; {group.totalRam.toFixed(0)} MB &middot; {group.totalCpu.toFixed(1)}%
                </span>
              </td>
            </tr>
            {#if !collapsedGroups.has(group.name)}
              {#each group.procs as proc (proc.pid)}
                {@render processRow(proc)}
              {/each}
            {/if}
          {/if}
        {/each}
      {:else}
        {#each sorted as proc (proc.pid)}
          {@render processRow(proc)}
        {/each}
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
  tr:hover {
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
    height: 22px;
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
    width: 32%;
    min-width: 120px;
  }
  .col-pid {
    width: 55px;
    text-align: right;
  }
  .col-ram {
    width: 60px;
    text-align: right;
  }
  .col-cpu {
    width: 50px;
    text-align: right;
  }
  .col-uptime {
    width: 42px;
    text-align: right;
    color: var(--fg-dim);
  }
  .col-state {
    width: 24px;
    text-align: center;
    color: var(--fg-dim);
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
