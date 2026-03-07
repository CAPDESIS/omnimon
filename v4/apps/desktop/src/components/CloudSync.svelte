<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let key = $state('');
  let status = $state('');

  onMount(async () => {
    try {
      key = await invoke('get_cloud_key');
    } catch (e) {
      console.error('Failed to get cloud key', e);
    }
  });

  async function saveKey() {
    try {
      await invoke('save_cloud_key', { key });
      status = 'Key saved successfully!';
    } catch (e) {
      status = `Error: ${e}`;
    }
  }

  function syncNow() {
    status = 'Sync not implemented yet.';
  }
</script>

<div class="cloud-sync">
  <h3>CrabNebula Cloud Settings</h3>
  <div class="input-group">
    <label for="cloud-key">API Key:</label>
    <input type="password" id="cloud-key" bind:value={key} placeholder="Enter your CrabNebula API Key" />
  </div>
  <div class="actions">
    <button class="btn btn-accent" onclick={saveKey}>Save Key</button>
    <button class="btn" onclick={syncNow}>Sync Now</button>
  </div>
  {#if status}
    <p class="status">{status}</p>
  {/if}
</div>

<style>
  .cloud-sync {
    padding: 1rem;
    border: 1px solid var(--border, #333);
    border-radius: 4px;
    margin-top: 1rem;
    background: var(--bg-alt, #1a1a1a);
  }
  .cloud-sync h3 {
    margin-top: 0;
    margin-bottom: 1rem;
  }
  .input-group {
    margin-bottom: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .input-group input {
    padding: 0.5rem;
    border: 1px solid var(--border, #333);
    border-radius: 4px;
    background: var(--bg, #000);
    color: var(--fg, #fff);
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .status {
    margin-top: 1rem;
    font-size: 0.9rem;
    color: var(--fg-muted, #999);
  }
</style>