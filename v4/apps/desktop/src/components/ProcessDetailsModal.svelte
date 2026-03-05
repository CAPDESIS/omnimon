<script lang="ts">
  import type { ProcessEntry } from "../lib/types";

  interface Props {
    process: ProcessEntry;
    onclose: () => void;
  }

  let { process, onclose }: Props = $props();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="backdrop" onclick={onclose} role="dialog" aria-label="Process Details" tabindex="-1">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal" onclick={(e: MouseEvent) => e.stopPropagation()} role="document" tabindex="-1">
    <div class="header">
      <span class="title">{process.name}</span>
      <span class="pid">PID {process.pid}</span>
      <button class="close-btn" onclick={onclose} aria-label="Close">&times;</button>
    </div>
    <div class="body">
      <div class="row">
        <span class="label">Executable</span>
        <span class="value mono">{process.exec_name}</span>
      </div>
      <div class="row">
        <span class="label">PID</span>
        <span class="value mono">{process.pid}</span>
      </div>
      <div class="row">
        <span class="label">RAM</span>
        <span class="value mono">{process.ram_mb.toFixed(1)} MB</span>
      </div>
      <div class="row">
        <span class="label">CPU</span>
        <span class="value mono">{process.cpu_pct.toFixed(1)}%</span>
      </div>
      <div class="row">
        <span class="label">Uptime</span>
        <span class="value mono">{process.uptime || "—"}</span>
      </div>
      <div class="row">
        <span class="label">Group</span>
        <span class="value">{process.group || "—"}</span>
      </div>
      <div class="row">
        <span class="label">State</span>
        <span class="value mono">{process.state}</span>
      </div>
      <div class="row">
        <span class="label">System</span>
        <span class="value">{process.is_system ? "Yes" : "No"}</span>
      </div>
      <div class="row">
        <span class="label">Idle</span>
        <span class="value">{process.idle ? "Yes" : "No"}</span>
      </div>
    </div>
    <div class="footer">
      <span class="hint">Esc to close</span>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: var(--bg-alt);
    border: 1px solid var(--border);
    border-radius: 6px;
    width: 360px;
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }

  .title {
    font-weight: 700;
    font-size: 12px;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pid {
    color: var(--fg-dim);
    font-size: 10px;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    flex-shrink: 0;
  }

  .close-btn {
    width: 20px;
    height: 20px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--fg-dim);
    font-size: 16px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    line-height: 1;
  }
  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  .body {
    padding: 6px 0;
  }

  .row {
    display: flex;
    align-items: baseline;
    padding: 3px 10px;
    font-size: 11px;
    gap: 8px;
  }
  .row:hover {
    background: var(--bg-hover);
  }

  .label {
    width: 72px;
    flex-shrink: 0;
    color: var(--fg-dim);
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .value {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    word-break: break-all;
  }

  .mono {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }

  .footer {
    padding: 4px 10px;
    border-top: 1px solid var(--border);
    text-align: right;
  }

  .hint {
    font-size: 9px;
    color: var(--fg-dim);
  }
</style>
