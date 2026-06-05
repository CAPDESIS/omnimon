<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import Button from "./Button.svelte";
  import EmptyState from "./EmptyState.svelte";
  import { Skull } from "lucide-svelte";
  import { t } from "../lib/i18n";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  type ZombieReason = "cpu_sustained" | "ram_sustained" | "cpu_and_ram_sustained";

  interface ZombieCandidate {
    pid: number;
    name: string;
    execName: string;
    exePath: string | null;
    cpuPct: number;
    memoryBytes: number;
    ageSecs: number;
    reason: ZombieReason;
    startTime: number;
  }

  interface ZombieKillerConfig {
    enabled: boolean;
    cpuThresholdPct: number;
    ramThresholdBytes: number;
    minUptimeSecs: number;
    sustainedSecs: number;
    autoKill: boolean;
    neverKill: string[];
  }

  const SECONDS_IN_DAY = 24 * 60 * 60;

  let config = $state<ZombieKillerConfig>({
    enabled: true,
    cpuThresholdPct: 50,
    ramThresholdBytes: 0,
    minUptimeSecs: 7 * SECONDS_IN_DAY,
    sustainedSecs: 3600,
    autoKill: false,
    neverKill: [],
  });
  let zombies = $state<ZombieCandidate[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let newBlocklistEntry = $state("");
  let error = $state<string | null>(null);
  let unlisten: UnlistenFn | null = null;

  let minUptimeDays = $derived(Math.round(config.minUptimeSecs / SECONDS_IN_DAY));
  let sustainedMinutes = $derived(Math.round(config.sustainedSecs / 60));
  let ramThresholdMb = $derived(Math.round(config.ramThresholdBytes / 1_048_576));

  function reasonLabel(reason: ZombieReason): string {
    switch (reason) {
      case "cpu_sustained":
        return t("zombieKiller.reasonCpu");
      case "ram_sustained":
        return t("zombieKiller.reasonRam");
      case "cpu_and_ram_sustained":
        return t("zombieKiller.reasonCpuAndRam");
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1_048_576) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1_073_741_824) return `${(bytes / 1_048_576).toFixed(1)} MB`;
    return `${(bytes / 1_073_741_824).toFixed(2)} GB`;
  }

  function formatAge(secs: number): string {
    const days = Math.floor(secs / SECONDS_IN_DAY);
    const hours = Math.floor((secs % SECONDS_IN_DAY) / 3600);
    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h`;
    const mins = Math.floor(secs / 60);
    return `${mins}m`;
  }

  async function loadConfig() {
    try {
      config = await invoke<ZombieKillerConfig>("get_zombie_killer_config");
    } catch (e) {
      error = t("zombieKiller.errorLoadConfig", { error: String(e) });
    }
  }

  async function loadZombies() {
    try {
      zombies = await invoke<ZombieCandidate[]>("list_zombie_candidates");
    } catch (e) {
      error = t("zombieKiller.errorLoadZombies", { error: String(e) });
    }
  }

  async function saveConfig() {
    saving = true;
    error = null;
    try {
      const toSave: ZombieKillerConfig = {
        ...config,
        cpuThresholdPct: clampFinite(config.cpuThresholdPct, 1, 10_000, 50),
        minUptimeSecs: clampFinite(minUptimeDays, 1, 365, 7) * SECONDS_IN_DAY,
        sustainedSecs: clampFinite(sustainedMinutes, 1, 1440, 60) * 60,
        ramThresholdBytes: Math.max(0, ramThresholdMb) * 1_048_576,
      };
      await invoke("set_zombie_killer_config", { config: toSave });
      config = toSave;
    } catch (e) {
      error = t("zombieKiller.errorSave", { error: String(e) });
    } finally {
      saving = false;
    }
  }

  function clampFinite(value: number, min: number, max: number, fallback: number): number {
    if (!Number.isFinite(value)) return fallback;
    return Math.min(Math.max(value, min), max);
  }

  async function handleKillOne(pid: number) {
    error = null;
    try {
      await invoke("kill_zombie", { pid });
      zombies = zombies.filter((z) => z.pid !== pid);
    } catch (e) {
      error = t("zombieKiller.errorKillOne", { pid, error: String(e) });
    }
  }

  async function handleKillAll() {
    if (zombies.length === 0) return;
    error = null;
    try {
      await invoke("kill_all_zombies");
      zombies = [];
    } catch (e) {
      error = t("zombieKiller.errorKillAll", { error: String(e) });
    }
  }

  function addBlocklistEntry() {
    const entry = newBlocklistEntry.trim();
    if (!entry) return;
    if (config.neverKill.some((existing) => existing.toLowerCase() === entry.toLowerCase())) {
      newBlocklistEntry = "";
      return;
    }
    config.neverKill = [...config.neverKill, entry];
    newBlocklistEntry = "";
  }

  function removeBlocklistEntry(entry: string) {
    config.neverKill = config.neverKill.filter((e) => e !== entry);
  }

  onMount(async () => {
    loading = true;
    await Promise.all([loadConfig(), loadZombies()]);
    loading = false;
    try {
      unlisten = await listen<ZombieCandidate[]>("zombie-killer-update", (event) => {
        zombies = event.payload ?? [];
      });
    } catch (e) {
      console.error("[ZombieKiller] failed to subscribe to updates:", e);
    }
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });
</script>

<div
  class="zk-backdrop"
  role="presentation"
  onclick={(event: MouseEvent) => {
    if (event.target === event.currentTarget) onclose();
  }}
>
  <div class="zk-dialog" role="dialog" aria-modal="true" aria-labelledby="zk-title">
    <header class="zk-header">
      <div class="zk-title-wrap">
        <Skull size={20} aria-hidden="true" />
        <h2 id="zk-title">{t("zombieKiller.title")}</h2>
      </div>
      <Button
        variant="ghost"
        size="icon"
        type="button"
        onclick={onclose}
        aria-label={t("zombieKiller.close")}
      >×</Button>
    </header>

    <div class="zk-body">
      {#if error}
        <div class="zk-error" role="alert">{error}</div>
      {/if}

      {#if loading}
        <p class="zk-status">{t("zombieKiller.loading")}</p>
      {:else}
        <section class="zk-section">
          <h3>{t("zombieKiller.configuration")}</h3>
          <div class="zk-grid">
            <label class="zk-toggle">
              <input type="checkbox" bind:checked={config.enabled} />
              <span>{t("zombieKiller.enabled")}</span>
            </label>
            <label class="zk-toggle">
              <input type="checkbox" bind:checked={config.autoKill} />
              <span>{t("zombieKiller.autoKill")}</span>
            </label>
          </div>

          <div class="zk-grid">
            <label class="zk-field">
              <span>{t("zombieKiller.cpuThreshold")}</span>
              <input
                type="number"
                min="1"
                max="10000"
                step="1"
                bind:value={config.cpuThresholdPct}
              />
            </label>

            <label class="zk-field">
              <span>{t("zombieKiller.ramThreshold")}</span>
              <input
                type="number"
                min="0"
                step="64"
                value={ramThresholdMb}
                oninput={(e) => {
                  const v = Number((e.currentTarget as HTMLInputElement).value);
                  config.ramThresholdBytes = (Number.isFinite(v) ? Math.max(0, v) : 0) * 1_048_576;
                }}
              />
            </label>

            <label class="zk-field">
              <span>{t("zombieKiller.minUptime")}</span>
              <input
                type="number"
                min="1"
                max="365"
                step="1"
                value={minUptimeDays}
                oninput={(e) => {
                  const v = Number((e.currentTarget as HTMLInputElement).value);
                  config.minUptimeSecs = clampFinite(v, 1, 365, 7) * SECONDS_IN_DAY;
                }}
              />
            </label>

            <label class="zk-field">
              <span>{t("zombieKiller.sustained")}</span>
              <input
                type="number"
                min="1"
                max="1440"
                step="1"
                value={sustainedMinutes}
                oninput={(e) => {
                  const v = Number((e.currentTarget as HTMLInputElement).value);
                  config.sustainedSecs = clampFinite(v, 1, 1440, 60) * 60;
                }}
              />
            </label>
          </div>

          <div class="zk-blocklist">
            <span class="zk-field-label">{t("zombieKiller.neverKillLabel")}</span>
            <div class="zk-blocklist-add">
              <input
                type="text"
                placeholder={t("zombieKiller.neverKillPlaceholder")}
                bind:value={newBlocklistEntry}
                onkeydown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    addBlocklistEntry();
                  }
                }}
              />
              <Button variant="secondary" size="sm" type="button" onclick={addBlocklistEntry}>
                {t("zombieKiller.add")}
              </Button>
            </div>
            {#if config.neverKill.length > 0}
              <ul class="zk-tags">
                {#each config.neverKill as entry (entry)}
                  <li>
                    <span>{entry}</span>
                    <button
                      class="zk-tag-remove"
                      type="button"
                      aria-label={t("zombieKiller.removeTag", { entry })}
                      onclick={() => removeBlocklistEntry(entry)}
                    >×</button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>

          <div class="zk-actions-row">
            <Button variant="primary" type="button" disabled={saving} onclick={saveConfig}>
              {saving ? t("zombieKiller.savingConfig") : t("zombieKiller.saveConfig")}
            </Button>
          </div>
        </section>

        <section class="zk-section">
          <header class="zk-section-header">
            <h3>{t("zombieKiller.detectedProcesses", { count: zombies.length })}</h3>
            {#if zombies.length > 0}
              <Button variant="danger" size="sm" type="button" onclick={handleKillAll}>
                {t("zombieKiller.killAll")}
              </Button>
            {/if}
          </header>

          {#if zombies.length === 0}
            <EmptyState
              icon={Skull}
              title={t("zombieKiller.emptyTitle")}
              description={t("zombieKiller.emptyBody")}
            />
          {:else}
            <ul class="zk-list">
              {#each zombies as z (z.pid)}
                <li class="zk-item">
                  <div class="zk-item-main">
                    <span class="zk-name">{z.name}</span>
                    <span class="zk-pid">{t("zombieKiller.pidLabel", { pid: z.pid })}</span>
                  </div>
                  <div class="zk-item-meta">
                    <span>{reasonLabel(z.reason)}</span>
                    <span>{t("zombieKiller.cpuLabel", { cpu: z.cpuPct.toFixed(0) })}</span>
                    <span>{formatBytes(z.memoryBytes)}</span>
                    <span>{t("zombieKiller.ageLabel", { age: formatAge(z.ageSecs) })}</span>
                  </div>
                  <Button
                    variant="danger"
                    size="sm"
                    type="button"
                    onclick={() => handleKillOne(z.pid)}
                  >
                    {t("zombieKiller.kill")}
                  </Button>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {/if}
    </div>
  </div>
</div>

<style>
  .zk-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    z-index: 1000;
  }

  .zk-dialog {
    width: min(760px, 100%);
    max-height: min(86vh, 720px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 18px;
    background: var(--bg-primary);
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.45);
  }

  .zk-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .zk-title-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .zk-title-wrap h2 {
    margin: 0;
    font-size: calc(var(--base-font-size) * 1.2);
    font-weight: 600;
  }

  .zk-body {
    padding: 18px 20px 22px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }

  .zk-error {
    padding: 10px 14px;
    border-radius: 8px;
    border: 1px solid var(--danger);
    background: var(--bg-secondary);
    color: var(--danger);
    font-size: calc(var(--base-font-size) * 0.95);
  }

  .zk-status {
    margin: 0;
    color: var(--text-secondary);
  }

  .zk-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .zk-section h3 {
    margin: 0;
    font-size: calc(var(--base-font-size) * 1.05);
    font-weight: 600;
  }

  .zk-section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .zk-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 10px;
  }

  .zk-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: calc(var(--base-font-size) * 0.95);
  }

  .zk-field-label {
    font-size: calc(var(--base-font-size) * 0.95);
    color: var(--text-secondary);
  }

  .zk-field span {
    color: var(--text-secondary);
  }

  .zk-field input,
  .zk-blocklist-add input {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 8px 12px;
    border-radius: 6px;
    font-size: var(--base-font-size);
  }

  .zk-toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    user-select: none;
  }

  .zk-blocklist {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .zk-blocklist-add {
    display: flex;
    gap: 8px;
  }

  .zk-blocklist-add input {
    flex: 1;
  }

  .zk-tags {
    list-style: none;
    padding: 0;
    margin: 4px 0 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .zk-tags li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    font-size: calc(var(--base-font-size) * 0.9);
  }

  .zk-tag-remove {
    background: var(--bg-alt);
    border: none;
    color: var(--text-secondary);
    width: 18px;
    height: 18px;
    border-radius: 999px;
    cursor: pointer;
    line-height: 1;
    font-size: 14px;
  }

  .zk-actions-row {
    display: flex;
    justify-content: flex-end;
  }

  .zk-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .zk-item {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px 12px;
    align-items: center;
    padding: 12px 14px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .zk-item-main {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .zk-name {
    font-weight: 600;
    word-break: break-all;
  }

  .zk-pid {
    font-size: calc(var(--base-font-size) * 0.85);
    color: var(--text-secondary);
  }

  .zk-item-meta {
    grid-column: 1 / -1;
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    color: var(--text-secondary);
    font-size: calc(var(--base-font-size) * 0.9);
  }
</style>
