<script lang="ts">
  import type { NetworkConnection } from "../../lib/types";
  import { fade } from "svelte/transition";
  import Button from "../Button.svelte";
  import IconButton from "../IconButton.svelte";

  interface Props {
    nodeId: string; // The hostname or IP
    connections: NetworkConnection[];
    onClose: () => void;
    onAskAi: (hostname: string) => void;
  }

  let { nodeId, connections, onClose, onAskAi }: Props = $props();

  let totalBytes = $derived(connections.reduce((sum, c) => sum + c.bytes_recv + c.bytes_sent, 0));
  let processes = $derived([...new Set(connections.map(c => c.process_name))].join(", "));
  let protocols = $derived([...new Set(connections.map(c => c.protocol.toUpperCase()))].join(", "));
  let ports = $derived([...new Set(connections.map(c => c.remote_port))].join(", "));

  function formatBytes(bytes: number) {
    if (bytes > 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(2) + " MB";
    if (bytes > 1024) return (bytes / 1024).toFixed(1) + " KB";
    return bytes + " B";
  }
</script>

<div class="connection-detail" transition:fade={{ duration: 150 }}>
  <div class="detail-header">
    <h3>{nodeId}</h3>
    <IconButton onclick={onClose} ariaLabel="Close" title="Close">×</IconButton>
  </div>
  
  <div class="detail-body">
    <div class="info-row">
      <span class="label">Procesos:</span>
      <span class="value">{processes}</span>
    </div>
    <div class="info-row">
      <span class="label">Protocolos:</span>
      <span class="value">{protocols}</span>
    </div>
    <div class="info-row">
      <span class="label">Puertos:</span>
      <span class="value">{ports}</span>
    </div>
    <div class="info-row">
      <span class="label">Tráfico total:</span>
      <span class="value">{formatBytes(totalBytes)}</span>
    </div>
    <div class="info-row">
      <span class="label">GeoIP / Región:</span>
      <span class="value">Desconocido (N/A)</span>
    </div>
    <div class="info-row">
      <span class="label">Duración:</span>
      <span class="value">Activa</span>
    </div>
  </div>

  <div class="detail-actions">
    <Button variant="primary" size="sm" onclick={() => onAskAi(nodeId)}>¿Qué es esto?</Button>
    <Button variant="danger" size="sm" onclick={() => alert("Función de bloqueo (placeholder)")}>Bloquear</Button>
  </div>
</div>

<style>
  .connection-detail {
    background: var(--bg-surface, #1e1e1e);
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  }

  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border, #333);
    padding-bottom: 8px;
  }

  .detail-header h3 {
    margin: 0;
    font-size: 14px;
    color: var(--fg, #fff);
  }

  .detail-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
  }

  .info-row {
    display: flex;
    justify-content: space-between;
  }

  .label {
    color: var(--fg-dim, #aaa);
  }

  .value {
    color: var(--fg, #fff);
    font-family: monospace;
    text-align: right;
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
</style>