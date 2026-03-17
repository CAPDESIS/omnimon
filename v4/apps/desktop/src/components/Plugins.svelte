<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../lib/i18n";
  import type { PluginDescriptor } from "../lib/types";
  import {
    ipcInstallPlugin,
    ipcListPlugins,
    ipcRemovePlugin,
    ipcSetPluginEnabled,
  } from "../lib/ipc";
  import Button from "./Button.svelte";
  import EmptyState from "./EmptyState.svelte";
  import { Loader2, Puzzle } from "lucide-svelte";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  let plugins = $state<PluginDescriptor[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let uploadState = $state<string | null>(null);
  let uploadBusy = $state(false);
  let refreshing = $state(false);
  let fileInput: HTMLInputElement | undefined = $state();
  function highlightLua(code: string): string {
    const keywords = /\b(function|return|end|local|if|then|else|elseif|for|while|do|repeat|until|in|and|or|not|true|false|nil)\b/g;
    const strings = /("(?:[^"\\]|\\.)*")/g;
    const numbers = /\b(\d+(?:\.\d+)?)\b/g;
    const comments = /(--[^\n]*)/g;

    let result = code
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");

    result = result.replace(comments, '<span class="lua-comment">$1</span>');
    result = result.replace(strings, '<span class="lua-string">$1</span>');
    result = result.replace(keywords, '<span class="lua-keyword">$1</span>');
    result = result.replace(numbers, '<span class="lua-number">$1</span>');
    return result;
  }

  const exampleScript = `function manifest()
  return {
    name = "Docker Monitor",
    version = "1.0.0",
    description = "Reports custom Docker metrics"
  }
end

function collect(ctx)
  return {
    metrics = {
      {
        name = "docker.containers.running",
        label = "Running containers",
        kind = "gauge",
        value = 0,
        unit = "count",
        tags = { source = "demo" }
      }
    }
  }
end`;

  const highlightedExample = highlightLua(exampleScript);

  const refreshIntervalMs = 4000;

  function formatRelativeTime(timestamp: number | null): string {
    if (!timestamp) return t("plugins.neverRun");
    const deltaSeconds = Math.max(0, Math.round((Date.now() - timestamp) / 1000));
    if (deltaSeconds < 5) return t("plugins.justNow");
    if (deltaSeconds < 60) return t("plugins.secondsAgo", { count: deltaSeconds });
    const minutes = Math.round(deltaSeconds / 60);
    if (minutes < 60) return t("plugins.minutesAgo", { count: minutes });
    const hours = Math.round(minutes / 60);
    return t("plugins.hoursAgo", { count: hours });
  }

  function metricValue(plugin: PluginDescriptor): number {
    return plugin.metrics.length;
  }

  async function loadPlugins(showLoading = false) {
    if (showLoading) loading = true;
    refreshing = !showLoading;
    error = null;
    try {
      plugins = await ipcListPlugins();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  async function handleUpload(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    uploadBusy = true;
    uploadState = null;
    error = null;
    try {
      const source = await file.text();
      await ipcInstallPlugin(file.name, source);
      uploadState = t("plugins.uploadSuccess", { name: file.name });
      await loadPlugins();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      uploadBusy = false;
      input.value = "";
    }
  }

  async function togglePlugin(plugin: PluginDescriptor) {
    error = null;
    try {
      await ipcSetPluginEnabled(plugin.id, !plugin.enabled);
      await loadPlugins();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  async function removePlugin(plugin: PluginDescriptor) {
    error = null;
    try {
      await ipcRemovePlugin(plugin.id);
      await loadPlugins();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(() => {
    let cancelled = false;
    loadPlugins(true);
    const interval = window.setInterval(() => {
      if (!cancelled) loadPlugins();
    }, refreshIntervalMs);

    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  });
</script>

<div
  class="plugins-backdrop"
  role="presentation"
  onclick={(event: MouseEvent) => {
    if (event.target === event.currentTarget) onclose();
  }}
>
  <div class="plugins-dialog" role="dialog" aria-modal="true" aria-labelledby="plugins-title">
    <header class="plugins-header">
      <div>
        <p class="plugins-eyebrow">{t("plugins.eyebrow")}</p>
        <h2 id="plugins-title">{t("plugins.title")}</h2>
        <p class="plugins-subtitle">{t("plugins.subtitle")}</p>
      </div>
      <Button class="close-btn" variant="ghost" size="icon" type="button" onclick={onclose} aria-label={t("common.close")}>x</Button>
    </header>

    <div class="plugins-toolbar">
      <div class="upload-card">
        <div>
          <strong>{t("plugins.uploadTitle")}</strong>
          <p>{t("plugins.uploadBody")}</p>
        </div>
        <input bind:this={fileInput} class="hidden-input" type="file" accept=".lua" onchange={handleUpload} />
        <Button variant="primary" type="button" onclick={() => fileInput?.click()} disabled={uploadBusy}>
          {uploadBusy ? t("plugins.uploading") : t("plugins.uploadButton")}
        </Button>
      </div>

      <div class="summary-card">
        <span>{t("plugins.installed", { count: plugins.length })}</span>
        <span>{t("plugins.active", { count: plugins.filter((plugin) => plugin.enabled).length })}</span>
        <span>{t("plugins.metricsCollected", { count: plugins.reduce((acc, plugin) => acc + metricValue(plugin), 0) })}</span>
        {#if refreshing}<span>{t("plugins.refreshing")}</span>{/if}
      </div>
    </div>

    {#if uploadState}
      <div class="banner success">{uploadState}</div>
    {/if}
    {#if error}
      <div class="banner error">{error}</div>
    {/if}

    <div class="plugins-body">
      {#if loading}
        <EmptyState icon={Loader2} title={t("common.loading")} description="" />
      {:else if plugins.length === 0}
        <EmptyState icon={Puzzle} title={t("plugins.emptyTitle")} description={t("plugins.emptyBody")}>
          <pre class="lua-code">{@html highlightedExample}</pre>
        </EmptyState>
      {:else}
        <div class="plugin-grid">
          {#each plugins as plugin (plugin.id)}
            <article class="plugin-card">
              <div class="plugin-topline">
                <div>
                  <h3>{plugin.name}</h3>
                  <p>{plugin.description ?? t("plugins.noDescription")}</p>
                </div>
                <span class:ok={plugin.status === "ok"} class:error={plugin.status === "error"} class:disabled={!plugin.enabled} class="status-pill">
                  {plugin.enabled ? plugin.status : t("plugins.disabled")}
                </span>
              </div>

              <div class="plugin-meta">
                <span>{plugin.file_name}</span>
                <span>{plugin.version ?? t("plugins.noVersion")}</span>
                <span>{t("plugins.lastRun")}: {formatRelativeTime(plugin.last_run_ms)}</span>
                <span>{t("plugins.duration")}: {plugin.last_duration_ms ?? 0}ms</span>
              </div>

              {#if plugin.last_error}
                <div class="error-box">{plugin.last_error}</div>
              {/if}

              <div class="metrics-section">
                <div class="metrics-header">
                  <strong>{t("plugins.metrics")}</strong>
                  <span>{plugin.metrics.length}</span>
                </div>
                {#if plugin.metrics.length === 0}
                  <p class="metrics-empty">{t("plugins.noMetrics")}</p>
                {:else}
                  <div class="metric-grid">
                    {#each plugin.metrics as metric (`${plugin.id}-${metric.name}`)}
                      <div class="metric-card">
                        <span class="metric-label">{metric.label}</span>
                        <strong>{metric.value}{metric.unit ? ` ${metric.unit}` : ""}</strong>
                        <span class="metric-kind">{metric.kind}</span>
                        {#if Object.keys(metric.tags).length > 0}
                          <div class="metric-tags">
                            {#each Object.entries(metric.tags) as [key, value] (`${plugin.id}-${metric.name}-${key}`)}
                              <span>{key}:{value}</span>
                            {/each}
                          </div>
                        {/if}
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>

              <div class="plugin-actions">
                <Button variant="secondary" type="button" onclick={() => togglePlugin(plugin)}>
                  {plugin.enabled ? t("plugins.disable") : t("plugins.enable")}
                </Button>
                <Button variant="danger" type="button" onclick={() => removePlugin(plugin)}>{t("plugins.remove")}</Button>
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .plugins-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    z-index: 1000;
  }

  .plugins-dialog {
    width: min(1180px, 100%);
    max-height: min(92vh, 920px);
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--accent) 20%, var(--border));
    border-radius: 24px;
    background: var(--bg-surface, var(--bg-alt));
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.45);
    padding: 20px;
  }

  .plugins-header,
  .plugins-toolbar,
  .plugin-topline,
  .plugin-meta,
  .metrics-header,
  .plugin-actions,
  .summary-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .plugins-eyebrow,
  .metric-kind,
  .status-pill {
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 11px;
    font-weight: 800;
  }

  .plugins-eyebrow,
  .metric-kind {
    color: var(--accent);
  }

  h2,
  h3,
  p {
    margin: 0;
  }

  .plugins-subtitle,
  .plugin-topline p,
  .metrics-empty,
  .summary-card,
  .plugin-meta {
    color: var(--fg-dim);
  }

  :global(.close-btn) {
    border-radius: 999px;
  }

  .upload-card,
  .summary-card,
  .plugin-card,
  .metric-card {
    border: 1px solid var(--border);
    border-radius: 18px;
    background: color-mix(in srgb, var(--bg-surface, var(--bg-alt)) 95%, white 2%);
  }

  .upload-card,
  .summary-card {
    padding: 14px 16px;
  }

  .upload-card {
    flex: 2 1 420px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .summary-card {
    flex: 1 1 280px;
  }

  .hidden-input {
    display: none;
  }

  .banner {
    border-radius: 14px;
    padding: 10px 12px;
    font-size: 13px;
  }

  .banner.success {
    background: color-mix(in srgb, var(--green) 28%, var(--bg));
    color: color-mix(in srgb, var(--green) 70%, white 12%);
  }

  .banner.error,
  .error-box {
    background: color-mix(in srgb, var(--danger) 28%, var(--bg));
    color: color-mix(in srgb, var(--danger) 78%, white 8%);
  }

  .plugins-body {
    overflow: auto;
    padding-right: 4px;
  }

  .plugin-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 14px;
  }

  .plugin-card {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .status-pill {
    border-radius: 999px;
    padding: 7px 10px;
    background: color-mix(in srgb, var(--bg) 92%, white 4%);
    color: var(--fg-dim);
  }

  .status-pill.ok {
    color: color-mix(in srgb, var(--green) 82%, white 8%);
  }

  .status-pill.error {
    color: color-mix(in srgb, var(--danger) 82%, white 8%);
  }

  .status-pill.disabled {
    color: var(--fg-muted);
  }

  .metrics-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .metric-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 10px;
  }

  .metric-card {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .metric-label {
    font-weight: 700;
  }

  .metric-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .metric-tags span {
    font-size: 11px;
    padding: 4px 6px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent) 25%, var(--bg));
    color: var(--fg-dim);
  }

  pre {
    width: 100%;
    overflow: auto;
    margin: 0;
    padding: 16px 18px;
    border-radius: 14px;
    background: #0d1117;
    color: #c9d1d9;
    font-family: "SF Mono", "Menlo", "Consolas", "Liberation Mono", monospace;
    font-size: 13px;
    line-height: 1.6;
    text-align: left;
    tab-size: 2;
  }

  pre :global(.lua-keyword) {
    color: #ff7b72;
    font-weight: 600;
  }

  pre :global(.lua-string) {
    color: #a5d6ff;
  }

  pre :global(.lua-number) {
    color: #79c0ff;
  }

  pre :global(.lua-comment) {
    color: #8b949e;
    font-style: italic;
  }

  @media (max-width: 720px) {
    .plugins-backdrop {
      padding: 12px;
    }

    .plugins-dialog {
      padding: 14px;
      border-radius: 20px;
    }

    .plugin-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
