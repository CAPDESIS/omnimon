<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  import Button from "./Button.svelte";

  let key = $state("");
  let status = $state("");
  let loadingKey = $state(false);
  let savingKey = $state(false);

  onMount(() => {
    let mounted = true;

    const loadKey = async () => {
      loadingKey = true;
      try {
        const stored = await invoke<string>('get_cloud_key');
        if (!mounted) return;
        key = stored;
      } catch (e) {
        if (!mounted) return;
        console.error('Failed to get cloud key', e);
      } finally {
        if (mounted) {
          loadingKey = false;
        }
      }
    };

    void loadKey();

    return () => {
      mounted = false;
    };
  });

  async function saveKey() {
    if (savingKey || loadingKey) return;
    savingKey = true;
    status = '';
    try {
      await invoke<void>('save_cloud_key', { key });
       status = "Key saved successfully!";
     } catch (e) {
       status = `Error: ${e}`;
    } finally {
      savingKey = false;
    }
  }

  function syncNow() {
     status = "Sync not implemented yet.";
  }
</script>

<div class="cloud-sync">
  <h3>CrabNebula Cloud Settings</h3>
  <div class="input-group">
     <label for="cloud-key">API Key:</label>
    <input
      type="password"
      id="cloud-key"
      bind:value={key}
      placeholder="Enter your CrabNebula API Key"
      disabled={loadingKey || savingKey}
    />
  </div>
  <div class="actions">
    <Button variant="primary" onclick={saveKey} disabled={loadingKey || savingKey || !key.trim()}>
       {savingKey ? "Saving..." : "Save Key"}
     </Button>
     <Button variant="secondary" onclick={syncNow} disabled={loadingKey || savingKey}>Sync Now</Button>
  </div>
  {#if status}
    <p class="status">{status}</p>
  {/if}
</div>

<style>
  .cloud-sync {
    padding: 1rem;
    border: 1px solid var(--border, #333);
    border-radius: var(--radius-md, 8px);
    margin-top: 1rem;
    background: var(--bg-surface, var(--bg-alt, #1a1a1a));
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
    padding: 0.625rem 0.75rem;
    border: 1px solid var(--border, #333);
    border-radius: var(--radius-md, 8px);
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
