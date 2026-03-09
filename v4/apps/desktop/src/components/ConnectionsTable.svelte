<script lang="ts">
  import {
    getNetworkState,
    getFilteredConnections,
  } from "../stores/network.svelte";

  const networkState = getNetworkState();

  let sortColumn = $state("bytes_up");
  let sortDirection = $state(-1); // -1 for desc, 1 for asc
  let expandedRow = $state<number | null>(null);

  function toggleSort(col: string) {
    if (sortColumn === col) {
      sortDirection *= -1;
    } else {
      sortColumn = col;
      sortDirection = -1;
    }
  }

  const sortedConnections = $derived.by(() => {
    const connections = getFilteredConnections();
    return [...connections].sort((a, b) => {
      let valA = a[sortColumn as keyof typeof a];
      let valB = b[sortColumn as keyof typeof b];

      if (typeof valA === "string" && typeof valB === "string") {
        return valA.localeCompare(valB) * sortDirection;
      }
      if (typeof valA === "number" && typeof valB === "number") {
        return (valA - valB) * sortDirection;
      }
      return 0;
    });
  });

  function formatSpeed(bytesPerSec: number) {
    if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`;
    if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
    return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
  }

  function getSpeedColor(bytesPerSec: number) {
    if (bytesPerSec > 1024 * 1024) return "text-red-500"; // > 1MB/s
    if (bytesPerSec > 100 * 1024) return "text-yellow-500"; // > 100KB/s
    return "text-green-500";
  }
</script>

<div class="connections-table-container">
  <div class="network-filters">
    <select bind:value={networkState.filter.protocol}>
      <option value="">Todos los protocolos</option>
      <option value="TCP">TCP</option>
      <option value="UDP">UDP</option>
    </select>

    <input type="text" placeholder="Filtrar por proceso..." bind:value={networkState.filter.process} />
    <input type="text" placeholder="Filtrar por dominio/IP..." bind:value={networkState.filter.host} />

    <label>
      <input type="checkbox" bind:checked={networkState.filter.hideLocalhost} />
      Ocultar localhost
    </label>

    <label>
      <input type="checkbox" bind:checked={networkState.filter.onlyEstablished} />
      Solo establecidas
    </label>

    <input type="number" placeholder="Min KB/s" bind:value={networkState.filter.minSpeed} />
  </div>

  <table class="connections-table">
    <thead>
      <tr>
        <th onclick={() => toggleSort('process_name')}>Proceso</th>
        <th onclick={() => toggleSort('protocol')}>Protocolo</th>
        <th onclick={() => toggleSort('local_address')}>Local</th>
        <th onclick={() => toggleSort('remote_address')}>Remoto</th>
        <th onclick={() => toggleSort('remote_hostname')}>Hostname</th>
        <th onclick={() => toggleSort('bytes_per_sec_up')}>↑ Speed</th>
        <th onclick={() => toggleSort('bytes_per_sec_down')}>↓ Speed</th>
        <th onclick={() => toggleSort('state')}>Estado</th>
      </tr>
    </thead>
    <tbody>
      {#each sortedConnections as conn}
        <tr onclick={() => expandedRow = expandedRow === conn.process_id ? null : conn.process_id}>
          <td>{conn.process_name || 'Unknown'} ({conn.process_id})</td>
          <td>{conn.protocol}</td>
          <td>{conn.local_address}:{conn.local_port}</td>
          <td>{conn.remote_address}:{conn.remote_port}</td>
          <td>{conn.remote_hostname}</td>
          <td class={getSpeedColor(conn.bytes_per_sec_up)}>{formatSpeed(conn.bytes_per_sec_up)}</td>
          <td class={getSpeedColor(conn.bytes_per_sec_down)}>{formatSpeed(conn.bytes_per_sec_down)}</td>
          <td>{conn.state}</td>
        </tr>
        {#if expandedRow === conn.process_id}
          <tr class="expanded-row">
            <td colspan="8">
              <div class="expanded-details">
                <p><strong>Total Up:</strong> {(conn.bytes_up / 1024).toFixed(2)} KB</p>
                <p><strong>Total Down:</strong> {(conn.bytes_down / 1024).toFixed(2)} KB</p>
              </div>
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
</div>

<style>
  .connections-table-container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    height: 100%;
  }

  .network-filters {
    display: flex;
    gap: 1rem;
    align-items: center;
    flex-wrap: wrap;
    padding: 0.5rem;
    background: var(--bg-surface, #f5f5f5);
    border-radius: 4px;
  }

  .connections-table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
  }

  .connections-table th {
    cursor: pointer;
    padding: 0.5rem;
    background: var(--bg-header, #e0e0e0);
  }

  .connections-table td {
    padding: 0.5rem;
    border-bottom: 1px solid var(--border-color, #ccc);
  }

  .connections-table tr:hover td {
    background: var(--bg-hover, #f0f0f0);
    cursor: pointer;
  }

  .text-red-500 { color: #ef4444; }
  .text-yellow-500 { color: #eab308; }
  .text-green-500 { color: #22c55e; }

  .expanded-row td {
    background: var(--bg-expanded, #fafafa);
    padding: 1rem;
    border-bottom: 2px solid var(--border-color, #ccc);
  }
</style>
