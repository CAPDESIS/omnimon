<script lang="ts">
  import type { ProcessEntry } from "../lib/types";
  import { toggleSelect, selectedPids } from "../stores/processes";

  interface Props {
    processes: ProcessEntry[];
  }

  let { processes }: Props = $props();

  type SortKey = "name" | "pid" | "ram_mb" | "cpu_pct" | "group";
  let sortKey: SortKey = $state("ram_mb");
  let sortAsc = $state(false);

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
</script>

<div class="table-wrap">
  <table>
    <thead>
      <tr>
        <th class="col-check"></th>
        <th class="col-name sortable" onclick={() => setSort("name")}>
          Name{arrow("name")}
        </th>
        <th class="col-pid sortable" onclick={() => setSort("pid")}>
          PID{arrow("pid")}
        </th>
        <th class="col-ram sortable" onclick={() => setSort("ram_mb")}>
          RAM{arrow("ram_mb")}
        </th>
        <th class="col-cpu sortable" onclick={() => setSort("cpu_pct")}>
          CPU{arrow("cpu_pct")}
        </th>
        <th class="col-group sortable" onclick={() => setSort("group")}>
          Group{arrow("group")}
        </th>
        <th class="col-state">St</th>
      </tr>
    </thead>
    <tbody>
      {#each sorted as proc (proc.pid)}
        <tr
          class:selected={$selectedPids.has(proc.pid)}
          class:system={proc.is_system}
          onclick={() => toggleSelect(proc.pid)}
        >
          <td class="col-check">
            <input
              type="checkbox"
              checked={$selectedPids.has(proc.pid)}
              disabled={proc.is_system}
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
          <td class="col-group">{proc.group}</td>
          <td class="col-state mono">{proc.state}</td>
        </tr>
      {/each}
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
  tr.system {
    opacity: 0.45;
  }
  tr.system:hover {
    opacity: 0.65;
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
    width: 60px;
    text-align: right;
  }
  .col-ram {
    width: 65px;
    text-align: right;
  }
  .col-cpu {
    width: 55px;
    text-align: right;
  }
  .col-group {
    width: 90px;
  }
  .col-state {
    width: 28px;
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
</style>
