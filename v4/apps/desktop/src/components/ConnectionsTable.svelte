<script lang="ts">
  import { t } from "../lib/i18n";
  import { formatConnectionState, formatProtocolLabel } from "../lib/localizedUi";
  import {
    getNetworkState,
    getFilteredConnections,
  } from "../stores/network.svelte";

  const networkState = getNetworkState();

  function updateProtocolFilter(event: Event) {
    networkState.filter.protocol = (event.currentTarget as HTMLSelectElement).value;
  }

  function updateProcessFilter(event: Event) {
    networkState.filter.process = (event.currentTarget as HTMLInputElement).value;
  }

  function updateHostFilter(event: Event) {
    networkState.filter.host = (event.currentTarget as HTMLInputElement).value;
  }

  function updateHideLocalhost(event: Event) {
    networkState.filter.hideLocalhost = (event.currentTarget as HTMLInputElement).checked;
  }

  function updateOnlyEstablished(event: Event) {
    networkState.filter.onlyEstablished = (event.currentTarget as HTMLInputElement).checked;
  }

  function updateMinSpeed(event: Event) {
    networkState.filter.minSpeed = Number((event.currentTarget as HTMLInputElement).value) || 0;
  }

  let sortColumn = $state("bytes_up");
  let sortDirection = $state(-1);
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
      const valA = a[sortColumn as keyof typeof a];
      const valB = b[sortColumn as keyof typeof b];

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
    if (bytesPerSec > 1024 * 1024) return "text-red-500";
    if (bytesPerSec > 100 * 1024) return "text-yellow-500";
    return "text-green-500";
  }
</script>

<div class="connections-table-container">
  <div class="network-filters">
    <select value={networkState.filter.protocol} onchange={updateProtocolFilter} aria-label={t("networkConnections.protocolFilter")}>
      <option value="">{t("networkConnections.allProtocols")}</option>
      <option value="TCP">{formatProtocolLabel("TCP")}</option>
      <option value="UDP">{formatProtocolLabel("UDP")}</option>
    </select>

    <input type="text" placeholder={t("networkConnections.filterByProcess")} value={networkState.filter.process} oninput={updateProcessFilter} />
    <input type="text" placeholder={t("networkConnections.filterByHost")} value={networkState.filter.host} oninput={updateHostFilter} />

    <label>
      <input type="checkbox" checked={networkState.filter.hideLocalhost} onchange={updateHideLocalhost} />
      {t("networkConnections.hideLocalhost")}
    </label>

    <label>
      <input type="checkbox" checked={networkState.filter.onlyEstablished} onchange={updateOnlyEstablished} />
      {t("networkConnections.onlyEstablished")}
    </label>

    <input type="number" placeholder={t("networkConnections.minSpeed")} value={networkState.filter.minSpeed} oninput={updateMinSpeed} />
  </div>

  <table class="connections-table">
    <thead>
      <tr>
        <th onclick={() => toggleSort("process_name")}>{t("networkConnections.columns.process")}</th>
        <th onclick={() => toggleSort("protocol")}>{t("networkConnections.columns.protocol")}</th>
        <th onclick={() => toggleSort("local_address")}>{t("networkConnections.columns.local")}</th>
        <th onclick={() => toggleSort("remote_address")}>{t("networkConnections.columns.remote")}</th>
        <th onclick={() => toggleSort("remote_hostname")}>{t("networkConnections.columns.hostname")}</th>
        <th onclick={() => toggleSort("bytes_per_sec_up")}>{t("networkConnections.columns.uploadSpeed")}</th>
        <th onclick={() => toggleSort("bytes_per_sec_down")}>{t("networkConnections.columns.downloadSpeed")}</th>
        <th onclick={() => toggleSort("state")}>{t("networkConnections.columns.state")}</th>
      </tr>
    </thead>
    <tbody>
      {#each sortedConnections as conn}
        <tr onclick={() => expandedRow = expandedRow === conn.process_id ? null : conn.process_id}>
          <td>{conn.process_name || t("networkConnections.unknownProcess")} ({conn.process_id})</td>
          <td>{formatProtocolLabel(conn.protocol)}</td>
          <td>{conn.local_address}:{conn.local_port}</td>
          <td>{conn.remote_address}:{conn.remote_port}</td>
          <td>{conn.remote_hostname}</td>
          <td class={getSpeedColor(conn.bytes_per_sec_up)}>{formatSpeed(conn.bytes_per_sec_up)}</td>
          <td class={getSpeedColor(conn.bytes_per_sec_down)}>{formatSpeed(conn.bytes_per_sec_down)}</td>
          <td>{formatConnectionState(conn.state)}</td>
        </tr>
        {#if expandedRow === conn.process_id}
          <tr class="expanded-row">
            <td colspan="8">
              <div class="expanded-details">
                <p><strong>{t("networkConnections.totalUp")}</strong> {(conn.bytes_up / 1024).toFixed(2)} KB</p>
                <p><strong>{t("networkConnections.totalDown")}</strong> {(conn.bytes_down / 1024).toFixed(2)} KB</p>
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
