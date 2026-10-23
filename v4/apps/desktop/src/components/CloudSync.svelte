<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { t } from "../lib/i18n";

  import Button from "./Button.svelte";

  type CloudSyncResult = { uploaded_bytes: number; sha256: string };

  let key = $state("");
  let status = $state("");
  let syncState = $state<"idle" | "success" | "error">("idle");
  let loadingKey = $state(false);
  let savingKey = $state(false);
  let syncing = $state(false);

  onMount(() => {
    let mounted = true;

    const loadKey = async () => {
      loadingKey = true;
      try {
        const stored = await invoke<string>("get_cloud_key");
        if (!mounted) return;
        key = stored;
      } catch (e) {
        if (!mounted) return;
        console.error("Failed to get cloud key", e);
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
    status = "";
    syncState = "idle";
    try {
      await invoke<void>("save_cloud_key", { key });
      status = t("cloudSync.saved");
      syncState = "success";
    } catch (e) {
      status = t("cloudSync.error", { error: e instanceof Error ? e.message : String(e) });
      syncState = "error";
    } finally {
      savingKey = false;
    }
  }

  async function syncNow() {
    if (syncing || loadingKey || savingKey) return;
    syncing = true;
    status = "";
    syncState = "idle";
    try {
      const result = await invoke<CloudSyncResult>("sync_cloud");
      syncState = "success";
      status = t("cloudSync.syncSuccess", { bytes: result.uploaded_bytes });
    } catch (e) {
      syncState = "error";
      status = t("cloudSync.error", { error: e instanceof Error ? e.message : String(e) });
    } finally {
      syncing = false;
    }
  }
</script>

<div class="cloud-sync">
  <h3>{t("cloudSync.title")}</h3>
  <div class="input-group">
     <label for="cloud-key">{t("cloudSync.apiKey")}</label>
    <input
      type="password"
      id="cloud-key"
      bind:value={key}
      placeholder={t("cloudSync.placeholder")}
      disabled={loadingKey || savingKey || syncing}
    />
  </div>
  <div class="actions">
    <Button variant="primary" onclick={saveKey} disabled={loadingKey || savingKey || syncing || !key.trim()}>
       {savingKey ? t("cloudSync.saving") : t("cloudSync.saveKey")}
      </Button>
      <Button variant="secondary" onclick={syncNow} disabled={loadingKey || savingKey || syncing}>{syncing ? t("cloudSync.syncing") : t("cloudSync.syncNow")}</Button>
   </div>
  {#if status}
    <p class="status" class:error={syncState === "error"} class:success={syncState === "success"} role="status" aria-live="polite">{status}</p>
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
  .status.error {
    color: var(--danger, #f87171);
  }
  .status.success {
    color: var(--success, #4ade80);
  }
</style>
