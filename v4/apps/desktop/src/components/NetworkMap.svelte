<script lang="ts">
  import ContextAiChat from "./ContextAiChat.svelte";
  import { tick } from "svelte";
  import { networkConnections, networkTelemetryStatus } from "../stores/security";
  import { metricsHistory } from "../stores/metricsHistory";
  import { theme } from "../stores/preferences";
  import { slide } from "svelte/transition";
  import type { MetricsSnapshot } from "../stores/metricsHistory";
  import type { NetworkConnection } from "../lib/types";
  import type { Time } from "lightweight-charts";
  import { t } from "../lib/i18n";
  import {
    NETWORK_PANEL_DEFAULT_HEIGHT,
    NETWORK_SIDE_PANEL_DEFAULT_WIDTH,
    NETWORK_CANVAS_LEFT_MARGIN,
    NETWORK_CANVAS_RIGHT_INSET,
    BYTES_PER_MB,
    BYTES_PER_KB,
  } from "../lib/constants";

  let collapsed = $state(true);
  let activeTab = $state<"map" | "table" | "traffic">("map");
  let panelHeight = $state(NETWORK_PANEL_DEFAULT_HEIGHT);
  let dragMode = $state<"content" | null>(null);
  let sidePanelWidth = $state(NETWORK_SIDE_PANEL_DEFAULT_WIDTH);
  let sideDragMode = $state<"sidebar" | null>(null);
  let dragStartY = 0;
  let dragStartHeight = NETWORK_PANEL_DEFAULT_HEIGHT;
  let dragStartX = 0;
  let dragStartWidth = NETWORK_SIDE_PANEL_DEFAULT_WIDTH;
  let chartLoadFailed = $state(false);
  let pendingChartInit = 0;

  // Group connections by process, then by domain
  interface ProcessNode {
    name: string;
    pid: number;
    domains: DomainNode[];
    totalConns: number;
  }

  interface DomainNode {
    hostname: string;
    port: number;
    protocol: string;
    count: number;
  }

  let processNodes = $derived.by((): ProcessNode[] => {
    const byProc = new Map<string, { pid: number; domains: Map<string, DomainNode> }>();

    for (const conn of $networkConnections) {
      const key = conn.process_name;
      if (!byProc.has(key)) {
        byProc.set(key, { pid: conn.pid, domains: new Map() });
      }
      const proc = byProc.get(key)!;
      const domKey = `${conn.remote_addr}:${conn.remote_port}`;
      const existing = proc.domains.get(domKey);
      if (existing) {
        existing.count++;
      } else {
        proc.domains.set(domKey, {
          hostname: conn.remote_addr,
          port: conn.remote_port,
          protocol: conn.protocol,
          count: 1,
        });
      }
    }

    return [...byProc.entries()]
      .map(([name, data]) => ({
        name,
        pid: data.pid,
        domains: [...data.domains.values()].sort((a, b) => b.count - a.count),
        totalConns: [...data.domains.values()].reduce((s, d) => s + d.count, 0),
      }))
      .sort((a, b) => b.totalConns - a.totalConns);
  });

  let totalConnections = $derived($networkConnections.length);
  let hasTrafficData = $derived($metricsHistory.length > 0 || $networkTelemetryStatus.totalRxBytesPerSec > 0 || $networkTelemetryStatus.totalTxBytesPerSec > 0);
  let hasAnyNetworkData = $derived(totalConnections > 0 || hasTrafficData);

  // --- Canvas-based connection map ---
  let canvas: HTMLCanvasElement | undefined = $state();

  $effect(() => {
    if (!canvas || collapsed || processNodes.length === 0 || activeTab !== "map") return;
    requestAnimationFrame(() => {
      if (canvas && !collapsed && activeTab === "map") {
        drawMap(canvas, processNodes);
      }
    });
  });

  function drawMap(cvs: HTMLCanvasElement, nodes: ProcessNode[]) {
    const ctx = cvs.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const w = cvs.clientWidth;
    const h = cvs.clientHeight;
    cvs.width = w * dpr;
    cvs.height = h * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);

    const getVar = (name: string) =>
      getComputedStyle(document.documentElement).getPropertyValue(name).trim();

    const fg = getVar("--fg") || "#ededef";
    const fgDim = getVar("--fg-dim") || "#71717a";
    const accent = getVar("--accent") || "#3b82f6";
    const green = getVar("--green") || "#22c55e";
    const border = getVar("--border") || "#27272a";

    const leftMargin = NETWORK_CANVAS_LEFT_MARGIN;
    const rightMargin = w - NETWORK_CANVAS_RIGHT_INSET;
    const nodeSpacing = Math.min(40, (h - 20) / Math.max(nodes.length, 1));

    ctx.font = `600 11px -apple-system, "SF Pro Text", sans-serif`;
    ctx.textBaseline = "middle";

    const allDomains = new Set<string>();
    for (const node of nodes) {
      for (const d of node.domains) {
        allDomains.add(d.hostname);
      }
    }
    const domainList = [...allDomains];
    const domainSpacing = Math.min(24, (h - 20) / Math.max(domainList.length, 1));

    for (let i = 0; i < nodes.length; i++) {
      const y = 14 + i * nodeSpacing;
      const node = nodes[i];

      ctx.fillStyle = fg;
      ctx.textAlign = "right";
      ctx.fillText(
        node.name.length > 14 ? node.name.slice(0, 13) + "\u2026" : node.name,
        leftMargin - 8,
        y,
      );

      ctx.beginPath();
      ctx.arc(leftMargin, y, 4, 0, Math.PI * 2);
      ctx.fillStyle = accent;
      ctx.fill();

      for (const domain of node.domains) {
        const domIdx = domainList.indexOf(domain.hostname);
        const dy = 14 + domIdx * domainSpacing;

        ctx.beginPath();
        ctx.moveTo(leftMargin + 4, y);
        const cx1 = leftMargin + (rightMargin - leftMargin) * 0.35;
        const cx2 = leftMargin + (rightMargin - leftMargin) * 0.65;
        ctx.bezierCurveTo(cx1, y, cx2, dy, rightMargin - 4, dy);
        ctx.strokeStyle = border;
        ctx.lineWidth = Math.min(domain.count, 3);
        ctx.globalAlpha = 0.4 + Math.min(domain.count * 0.1, 0.5);
        ctx.stroke();
        ctx.globalAlpha = 1;
      }
    }

    ctx.font = `400 10px "SF Mono", "Menlo", monospace`;
    for (let i = 0; i < domainList.length; i++) {
      const y = 14 + i * domainSpacing;
      const hostname = domainList[i];

      ctx.beginPath();
      ctx.arc(rightMargin, y, 3, 0, Math.PI * 2);
      ctx.fillStyle = green;
      ctx.fill();

      ctx.fillStyle = fgDim;
      ctx.textAlign = "left";
      const label = hostname.length > 20 ? hostname.slice(0, 19) + "\u2026" : hostname;
      ctx.fillText(label, rightMargin + 8, y);
    }
  }

  // --- Traffic area chart using lightweight-charts ---
  let trafficChartEl: HTMLDivElement | undefined = $state();
  type TrafficPoint = { time: Time; value: number };
  type TrafficSeriesApi = {
    setData: (data: TrafficPoint[]) => void;
    update: (point: TrafficPoint) => void;
  };
  type TrafficChartApi = {
    addSeries: (seriesType: unknown, options: Record<string, unknown>) => TrafficSeriesApi;
    timeScale: () => { fitContent: () => void };
    applyOptions: (options: { width: number }) => void;
    remove: () => void;
  };

  let chartInstance: TrafficChartApi | undefined = $state(undefined);
  let rxSeriesInstance: TrafficSeriesApi | undefined = $state(undefined);
  let txSeriesInstance: TrafficSeriesApi | undefined = $state(undefined);
  let trafficResizeObserver: ResizeObserver | undefined = $state(undefined);
  let lastTrafficPointTime = $state<number | null>(null);
  let lastTrafficHistoryIndex = $state(-1);

  $effect(() => {
    if (!trafficChartEl || collapsed || activeTab !== "traffic") return;
    const token = ++pendingChartInit;
    tick().then(() => {
      if (token !== pendingChartInit || !trafficChartEl || collapsed || activeTab !== "traffic") return;
      initTrafficChart(trafficChartEl);
    });
  });

  function toTrafficPoint(snapshot: MetricsSnapshot, direction: "rx" | "tx"): TrafficPoint {
    return {
      time: snapshot.time as Time,
      value: (direction === "rx" ? snapshot.netRx : snapshot.netTx) / BYTES_PER_KB,
    };
  }

  function resetTrafficSeries(history: MetricsSnapshot[]) {
    if (!rxSeriesInstance || !txSeriesInstance) return;
    const rxData = history.map((entry) => toTrafficPoint(entry, "rx"));
    const txData = history.map((entry) => toTrafficPoint(entry, "tx"));
    rxSeriesInstance.setData(rxData);
    txSeriesInstance.setData(txData);
    if (history.length > 0) {
      lastTrafficHistoryIndex = history.length - 1;
      lastTrafficPointTime = history[history.length - 1].time;
    } else {
      lastTrafficHistoryIndex = -1;
      lastTrafficPointTime = null;
    }
  }

  async function initTrafficChart(container: HTMLDivElement) {
    try {
      const lc = await import("lightweight-charts");
      if (chartInstance) return;
      chartLoadFailed = false;

      const getVar = (name: string) =>
        getComputedStyle(document.documentElement).getPropertyValue(name).trim();

      const chart = lc.createChart(container, {
        width: container.clientWidth,
        height: 180,
        layout: {
          background: { color: "transparent" } as any,
          textColor: getVar("--fg-dim") || "#71717a",
          fontSize: 10,
        },
        grid: {
          vertLines: { color: getVar("--border") || "#27272a" },
          horzLines: { color: getVar("--border") || "#27272a" },
        },
        timeScale: {
          timeVisible: true,
          secondsVisible: false,
        },
        rightPriceScale: {
          borderColor: getVar("--border") || "#27272a",
        },
      });

      const rxSeries = chart.addSeries(lc.AreaSeries, {
        lineColor: getVar("--chart-net-rx") || "#22c55e",
        topColor: (getVar("--chart-net-rx") || "#22c55e") + "40",
        bottomColor: (getVar("--chart-net-rx") || "#22c55e") + "05",
        lineWidth: 2,
        title: "RX",
      });

      const txSeries = chart.addSeries(lc.AreaSeries, {
        lineColor: getVar("--chart-net-tx") || "#f97316",
        topColor: (getVar("--chart-net-tx") || "#f97316") + "40",
        bottomColor: (getVar("--chart-net-tx") || "#f97316") + "05",
        lineWidth: 2,
        title: "TX",
      });

      rxSeriesInstance = rxSeries;
      txSeriesInstance = txSeries;
      resetTrafficSeries($metricsHistory);

      chart.timeScale().fitContent();
      chartInstance = chart as TrafficChartApi;

      const ro = new ResizeObserver(() => {
        chart.applyOptions({ width: container.clientWidth });
      });
      ro.observe(container);
      trafficResizeObserver = ro;
    } catch {
      chartLoadFailed = true;
    }
  }

  // Cleanup chart on collapse
  $effect(() => {
    if (collapsed && chartInstance) {
      try { chartInstance.remove(); } catch {}
      if (trafficResizeObserver) {
        trafficResizeObserver.disconnect();
        trafficResizeObserver = undefined;
      }
      chartInstance = undefined;
      rxSeriesInstance = undefined;
      txSeriesInstance = undefined;
      lastTrafficPointTime = null;
      lastTrafficHistoryIndex = -1;
      chartLoadFailed = false;
    }
  });

  // Recreate chart when theme changes
  $effect(() => {
    const _ = $theme; // subscribe to theme changes
    if (!trafficChartEl || collapsed || activeTab !== "traffic") return;
    if (chartInstance) {
      try { chartInstance.remove(); } catch {}
      if (trafficResizeObserver) {
        trafficResizeObserver.disconnect();
        trafficResizeObserver = undefined;
      }
      chartInstance = undefined;
      rxSeriesInstance = undefined;
      txSeriesInstance = undefined;
      lastTrafficPointTime = null;
      lastTrafficHistoryIndex = -1;
      chartLoadFailed = false;
    }
    // defer to allow CSS vars to update
    requestAnimationFrame(() => {
      if (trafficChartEl && !collapsed && activeTab === "traffic") {
        initTrafficChart(trafficChartEl);
      }
    });
  });

  // Push traffic updates in real time using snapshot.time (handles sleep/wake gaps)
  $effect(() => {
    const history = $metricsHistory;
    if (!rxSeriesInstance || !txSeriesInstance || !chartInstance) return;
    if (collapsed || activeTab !== "traffic") return;
    if (history.length === 0) {
      resetTrafficSeries(history);
      return;
    }

    const last = history[history.length - 1];

    const needsReset =
      lastTrafficHistoryIndex < 0 ||
      lastTrafficHistoryIndex >= history.length ||
      (lastTrafficHistoryIndex >= 0 &&
        history[lastTrafficHistoryIndex]?.time !== lastTrafficPointTime);

    if (needsReset) {
      resetTrafficSeries(history);
      chartInstance.timeScale().fitContent();
      return;
    }

    if (last.time === lastTrafficPointTime) {
      rxSeriesInstance.update(toTrafficPoint(last, "rx"));
      txSeriesInstance.update(toTrafficPoint(last, "tx"));
      return;
    }

    for (let i = lastTrafficHistoryIndex + 1; i < history.length; i++) {
      const point = history[i];
      rxSeriesInstance.update(toTrafficPoint(point, "rx"));
      txSeriesInstance.update(toTrafficPoint(point, "tx"));
      lastTrafficHistoryIndex = i;
      lastTrafficPointTime = point.time;
    }
  });

  // --- Connections table sort ---
  let tableSortKey = $state<"process" | "addr" | "port" | "proto" | "state" | "direction" | "bytes">("process");
  let tableSortAsc = $state(true);

  let sortedConnections = $derived.by(() => {
    const conns = [...$networkConnections];
    conns.sort((a, b) => {
      let va: string | number = "", vb: string | number = "";
      switch (tableSortKey) {
        case "process": va = a.process_name; vb = b.process_name; break;
        case "addr": va = a.remote_addr; vb = b.remote_addr; break;
        case "port": va = a.remote_port; vb = b.remote_port; break;
        case "proto": va = a.protocol; vb = b.protocol; break;
        case "state": va = a.state; vb = b.state; break;
        case "direction": va = a.direction; vb = b.direction; break;
        case "bytes": va = totalConnBytes(a); vb = totalConnBytes(b); break;
      }
      if (typeof va === "string" && typeof vb === "string") {
        return tableSortAsc ? va.localeCompare(vb) : vb.localeCompare(va);
      }
      return tableSortAsc ? Number(va) - Number(vb) : Number(vb) - Number(va);
    });
    return conns;
  });

  let visibleConnections = $derived(sortedConnections.slice(0, 100));

  function setTableSort(key: typeof tableSortKey) {
    if (tableSortKey === key) tableSortAsc = !tableSortAsc;
    else { tableSortKey = key; tableSortAsc = true; }
  }

  function sortArrow(key: typeof tableSortKey): string {
    if (tableSortKey !== key) return "";
    return tableSortAsc ? " \u25B2" : " \u25BC";
  }

  function clampPanelHeight(value: number): number {
    return Math.max(180, Math.min(value, 720));
  }

  function setPanelSize(delta: number) {
    panelHeight = clampPanelHeight(panelHeight + delta);
  }

  function startResize(event: MouseEvent) {
    event.preventDefault();
    dragMode = "content";
    dragStartY = event.clientY;
    dragStartHeight = panelHeight;
    window.addEventListener("mousemove", onResizeMove);
    window.addEventListener("mouseup", stopResize);
  }

  function startSideResize(event: MouseEvent) {
    event.preventDefault();
    sideDragMode = "sidebar";
    dragStartX = event.clientX;
    dragStartWidth = sidePanelWidth;
    window.addEventListener("mousemove", onSideResizeMove);
    window.addEventListener("mouseup", stopSideResize);
  }

  function onSideResizeMove(event: MouseEvent) {
    if (!sideDragMode) return;
    sidePanelWidth = Math.max(260, Math.min(dragStartWidth - (event.clientX - dragStartX), 520));
  }

  function stopSideResize() {
    sideDragMode = null;
    window.removeEventListener("mousemove", onSideResizeMove);
    window.removeEventListener("mouseup", stopSideResize);
  }

  function onResizeMove(event: MouseEvent) {
    if (!dragMode) return;
    panelHeight = clampPanelHeight(dragStartHeight + (event.clientY - dragStartY));
  }

  function stopResize() {
    dragMode = null;
    window.removeEventListener("mousemove", onResizeMove);
    window.removeEventListener("mouseup", stopResize);
  }

  function summarizeConnections(connections: NetworkConnection[]): Array<Record<string, unknown>> {
    const counts = new Map<string, { process: string; destination: string; count: number }>();
    for (const conn of connections) {
      const key = `${conn.process_name}:${conn.remote_addr}:${conn.remote_port}`;
      const current = counts.get(key);
      if (current) current.count += 1;
      else counts.set(key, {
        process: conn.process_name,
        destination: `${conn.remote_addr}:${conn.remote_port}`,
        count: 1,
      });
    }
    return [...counts.values()].sort((a, b) => Number(b.count) - Number(a.count)).slice(0, 12);
  }

  function buildNetworkContext(question: string): string {
    const recentTraffic = $metricsHistory.slice(-20);
    return JSON.stringify({
      prompt: question,
      active_tab: activeTab,
      summary: {
        total_connections: totalConnections,
        visible_processes: processNodes.length,
        panel_height: panelHeight,
        capture_backend: $networkTelemetryStatus.captureBackend,
        using_fallback: $networkTelemetryStatus.usingFallback,
      },
      connections: summarizeConnections($networkConnections),
      traffic: recentTraffic.map((entry) => ({
        time: entry.time,
        rx_kb_per_sec: Number((entry.netRx / BYTES_PER_KB).toFixed(2)),
        tx_kb_per_sec: Number((entry.netTx / BYTES_PER_KB).toFixed(2)),
      })),
    });
  }

  function formatRate(bytesPerSec: number): string {
    if (bytesPerSec >= BYTES_PER_MB) return `${(bytesPerSec / BYTES_PER_MB).toFixed(2)} MB/s`;
    if (bytesPerSec >= BYTES_PER_KB) return `${(bytesPerSec / BYTES_PER_KB).toFixed(1)} KB/s`;
    return `${bytesPerSec.toFixed(0)} B/s`;
  }

  function totalConnBytes(conn: NetworkConnection): number {
    return conn.bytes_sent + conn.bytes_recv;
  }

  function exportMapSnapshot() {
    if (!canvas) return;
    const link = document.createElement("a");
    link.href = canvas.toDataURL("image/png");
    link.download = `omnimon-network-map-${Date.now()}.png`;
    link.click();
  }

</script>

{#if hasAnyNetworkData}
<div class="netmap-section">
    <div class="netmap-toggle-wrap">
      <button
        class="netmap-toggle"
        aria-expanded={!collapsed}
        onclick={() => collapsed = !collapsed}
      >
      <span class="chevron" class:open={!collapsed}>&#9654;</span>
      <span class="netmap-title">{t("network.title")}</span>
      <span class="netmap-count">{t("network.summary", { connections: String(totalConnections), processes: String(processNodes.length) })}</span>
      </button>
      <span class="netmap-actions">
        <button class="size-btn" type="button" onclick={exportMapSnapshot} title={t("network.exportMap")}>⇩</button>
        <button class="size-btn" type="button" onclick={() => setPanelSize(-40)} title={t("common.smaller")}>−</button>
        <button class="size-btn" type="button" onclick={() => setPanelSize(40)} title={t("common.larger")}>+</button>
      </span>
    </div>

    {#if !collapsed}
      <div class="netmap-body" style={`height:${panelHeight}px`} transition:slide={{ duration: 200 }}>
        <!-- Tab bar -->
        <div class="tab-bar" role="tablist">
          <button
            class="tab-btn"
            class:active={activeTab === "map"}
            onclick={() => activeTab = "map"}
            role="tab"
            aria-selected={activeTab === "map"}
          >{t("network.map")}</button>
          <button
            class="tab-btn"
            class:active={activeTab === "table"}
            onclick={() => activeTab = "table"}
            role="tab"
            aria-selected={activeTab === "table"}
          >{t("network.connections")}</button>
          <button
            class="tab-btn"
            class:active={activeTab === "traffic"}
            onclick={() => activeTab = "traffic"}
            role="tab"
            aria-selected={activeTab === "traffic"}
          >{t("network.traffic")}</button>
        </div>

        <div class="tab-grid" style={`grid-template-columns:minmax(0,1fr) 6px minmax(260px, ${sidePanelWidth}px)`}>
          <div class="tab-main">
            {#if totalConnections === 0 && activeTab !== "traffic"}
              <div class="empty-state">{t("network.waiting")}</div>
            {/if}

            <!-- Map Tab -->
            {#if activeTab === "map"}
              <div class="tab-content map-content">
                {#if totalConnections === 0}
                  <div class="empty-state">{t("network.waiting")}</div>
                {:else}
                <canvas
                  bind:this={canvas}
                  class="netmap-canvas"
                  height={Math.min(processNodes.length * 40 + 20, Math.max(panelHeight - 120, 220))}
                ></canvas>
                <div class="netmap-list">
                  {#each processNodes as node (node.name)}
                    <div class="netmap-proc">
                      <span class="proc-name">{node.name}</span>
                      <span class="proc-count">{node.totalConns}</span>
                      <div class="domain-chips">
                        {#each node.domains.slice(0, 5) as domain}
                          <span class="domain-chip" title="{domain.hostname}:{domain.port} ({domain.protocol})">
                            {domain.hostname}:{domain.port}
                          </span>
                        {/each}
                        {#if node.domains.length > 5}
                          <span class="domain-more">+{node.domains.length - 5}</span>
                        {/if}
                      </div>
                    </div>
                  {/each}
                </div>
                {/if}
              </div>
            {/if}

            <!-- Connections Table Tab -->
            {#if activeTab === "table"}
              <div class="tab-content table-content">
                <div class="network-help">{t("network.connectionsHelp")}</div>
                {#if totalConnections === 0}
                  <div class="empty-state">{t("network.waiting")}</div>
                {:else}
                <table class="conn-table" aria-label="Active connections">
                  <thead>
                    <tr>
                      <th class="sortable" onclick={() => setTableSort("process")}>{t("network.process")}{sortArrow("process")}</th>
                      <th class="sortable" onclick={() => setTableSort("addr")}>{t("network.destination")}{sortArrow("addr")}</th>
                      <th class="sortable" onclick={() => setTableSort("port")}>{t("network.port")}{sortArrow("port")}</th>
                      <th class="sortable" onclick={() => setTableSort("direction")}>{t("network.direction")}{sortArrow("direction")}</th>
                      <th class="sortable" onclick={() => setTableSort("proto")}>{t("network.protocol")}{sortArrow("proto")}</th>
                      <th class="sortable" onclick={() => setTableSort("bytes")}>{t("network.bytes")}{sortArrow("bytes")}</th>
                      <th class="sortable" onclick={() => setTableSort("state")}>{t("network.state")}{sortArrow("state")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each visibleConnections as conn (conn.pid + conn.remote_addr + conn.remote_port + conn.direction)}
                      <tr>
                        <td class="col-process">{conn.process_name}</td>
                        <td class="col-addr mono">{conn.remote_addr}</td>
                        <td class="col-port mono">{conn.remote_port}</td>
                        <td class="col-direction mono">{conn.direction}</td>
                        <td class="col-proto mono">{conn.protocol.toUpperCase()}</td>
                        <td class="col-bytes mono">{formatRate(totalConnBytes(conn))}</td>
                        <td class="col-state mono">{conn.state}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
                {#if sortedConnections.length > 50}
                  <div class="table-overflow">{t("network.showingCount", { visible: String(visibleConnections.length), total: String(sortedConnections.length) })}</div>
                {/if}
                {/if}
              </div>
            {/if}

            <!-- Traffic Chart Tab -->
            {#if activeTab === "traffic"}
              <div class="tab-content traffic-content">
                <div class="network-help">{t("network.trafficHelp")}</div>
                <div class="traffic-topline">
                  <div class="traffic-stat"><span>RX</span><strong>{formatRate($networkTelemetryStatus.totalRxBytesPerSec)}</strong></div>
                  <div class="traffic-stat"><span>TX</span><strong>{formatRate($networkTelemetryStatus.totalTxBytesPerSec)}</strong></div>
                  <div class="traffic-stat"><span>{t("network.connections")}</span><strong>{totalConnections}</strong></div>
                </div>
                <div class="traffic-legend">
                  <span class="legend-item rx">&#9660; {t("network.inbound")}</span>
                  <span class="legend-item tx">&#9650; {t("network.outbound")}</span>
                </div>
                {#if chartLoadFailed}
                  <div class="traffic-fallback">{t("network.chartUnavailable")}</div>
                {:else if !hasTrafficData}
                  <div class="traffic-fallback">{t("network.waiting")}</div>
                {/if}
                <!-- Keep mounted to avoid expensive chart/container re-creation loops while switching tabs -->
                <div class="traffic-chart" class:hidden={chartLoadFailed || !hasTrafficData} bind:this={trafficChartEl}></div>
              </div>
            {/if}
          </div>

          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div class="side-resize-divider" class:active={sideDragMode === "sidebar"} onmousedown={startSideResize} role="separator" aria-orientation="vertical" aria-label={t("common.expand")}></div>

          <div class="tab-side">
            <div class="capture-chip">{t("network.captureBackend", { backend: $networkTelemetryStatus.captureBackend })}</div>
            <div class="capture-chip">RX {$networkTelemetryStatus.totalRxBytesPerSec > 0 ? formatRate($networkTelemetryStatus.totalRxBytesPerSec) : "0 B/s"}</div>
            <div class="capture-chip">TX {$networkTelemetryStatus.totalTxBytesPerSec > 0 ? formatRate($networkTelemetryStatus.totalTxBytesPerSec) : "0 B/s"}</div>
            <div class="network-help">{t("network.mapDeepInfo")}</div>
            {#if $networkTelemetryStatus.usingFallback}
              <div class="network-warning">{t("network.fallbackNotice")}</div>
            {/if}
            <ContextAiChat
              title={t("network.aiTitle")}
              placeholder={t("network.aiPlaceholder")}
              emptyState={t("network.aiEmpty")}
              helpTooltip={t("network.aiHelp")}
              sendLabel={t("common.askAi")}
              inputAriaLabel={t("network.aiTitle")}
              maxHeight={Math.max(panelHeight - 120, 180)}
              buildContext={buildNetworkContext}
            />
          </div>
        </div>

        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          class="netmap-resize-divider"
          class:active={dragMode === "content"}
          onmousedown={startResize}
          role="separator"
          aria-orientation="horizontal"
          aria-label={t("common.expand")}
          tabindex="-1"
        ></div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .netmap-section {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    background: var(--bg-alt);
  }

  .netmap-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    padding: 6px 10px;
    border: none;
    background: transparent;
    color: var(--fg);
    cursor: pointer;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    text-align: left;
  }
  .netmap-toggle:hover { background: var(--bg-hover); }

  .netmap-toggle-wrap {
    display: flex;
    align-items: stretch;
  }

  .netmap-actions {
    display: inline-flex;
    gap: 4px;
    margin-left: 8px;
  }

  .size-btn {
    width: 22px;
    height: 22px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    cursor: pointer;
    font-weight: 700;
  }

  .size-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .chevron {
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    color: var(--fg-dim);
    transition: transform 0.15s ease;
    display: inline-block;
  }
  .chevron.open { transform: rotate(90deg); }

  .netmap-title {
    font-weight: 700;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--chart-net-rx, var(--green));
  }

  .netmap-count {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--fg-dim);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    margin-left: auto;
  }

  .netmap-body {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .tab-grid {
    display: grid;
    min-height: 0;
    flex: 1;
  }

  .tab-main,
  .tab-side {
    min-height: 0;
  }

  .tab-side {
    border-left: 1px solid var(--border);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: rgba(0, 0, 0, 0.06);
  }

  .capture-chip {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--fg-dim);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }

  .network-warning,
  .traffic-fallback {
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--yellow);
    background: rgba(245, 158, 11, 0.08);
  }

  .side-resize-divider {
    width: 6px;
    cursor: ew-resize;
    background: var(--border);
    transition: background 0.15s ease;
  }

  .side-resize-divider:hover,
  .side-resize-divider.active {
    background: var(--accent);
  }

  .network-help,
  .empty-state {
    padding: 8px 10px;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--fg-dim);
  }

  .netmap-resize-divider {
    height: 4px;
    background: var(--border);
    cursor: ns-resize;
    transition: background 0.15s ease;
  }

  .netmap-resize-divider:hover,
  .netmap-resize-divider.active {
    background: var(--accent);
  }

  /* Tab bar */
  .tab-bar {
    display: flex;
    border-bottom: 1px solid var(--border);
    padding: 0 10px;
    gap: 0;
  }

  .tab-btn {
    padding: 5px 12px;
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color 0.15s, border-color 0.15s;
  }
  .tab-btn:hover { color: var(--fg); }
  .tab-btn.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .tab-content {
    overflow: auto;
    height: 100%;
  }

  /* Map content */
  .map-content {
    display: flex;
    gap: 0;
    height: 100%;
  }

  .netmap-canvas {
    flex: 1;
    min-width: 0;
    min-height: 220px;
  }

  .netmap-list {
    flex: 0 0 260px;
    overflow-y: auto;
    padding: 4px 8px;
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .netmap-proc {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    padding: 3px 0;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
  }

  .proc-name {
    font-weight: 600;
    min-width: 80px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .proc-count {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--fg-dim);
    min-width: 20px;
    text-align: right;
  }

  .domain-chips {
    display: flex;
    gap: 3px;
    flex-wrap: wrap;
    flex: 1;
    min-width: 0;
  }

  .domain-chip {
    display: inline-block;
    padding: 0 5px;
    border-radius: 3px;
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    background: var(--accent-dim, rgba(59,130,246,0.12));
    color: var(--accent);
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.6;
  }

  .domain-more {
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    color: var(--fg-dim);
    white-space: nowrap;
  }

  /* Connections Table */
  .table-content {
    height: 100%;
    overflow-y: auto;
  }

  .conn-table {
    width: 100%;
    border-collapse: collapse;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
  }

  .conn-table thead {
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .conn-table th {
    padding: 4px 8px;
    text-align: left;
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border);
    color: var(--fg-dim);
    font-weight: 600;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    white-space: nowrap;
    user-select: none;
  }
  .conn-table th.sortable { cursor: pointer; }
  .conn-table th.sortable:hover { color: var(--fg); }

  .conn-table td {
    padding: 3px 8px;
    border-bottom: 1px solid var(--border-subtle, rgba(128,128,128,0.1));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .conn-table tr:hover td { background: var(--bg-hover); }

  .conn-table .mono {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
  }

  .col-process { max-width: 120px; font-weight: 600; }
  .col-addr { max-width: 180px; }
  .col-port { width: 60px; text-align: right; }
  .col-direction { width: 88px; text-transform: uppercase; }
  .col-proto { width: 50px; text-align: center; }
  .col-bytes { width: 95px; text-align: right; }
  .col-state { width: 90px; color: var(--fg-dim); }

  .table-overflow {
    padding: 6px 8px;
    text-align: center;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--fg-dim);
    border-top: 1px solid var(--border);
  }

  /* Traffic Chart */
  .traffic-content {
    padding: 8px 10px;
    height: 100%;
  }

  .traffic-topline {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    margin: 0 0 8px;
  }

  .traffic-stat {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    background: rgba(255,255,255,0.02);
  }

  .traffic-stat span {
    font-size: calc(var(--base-font-size, 12px) * 0.72);
    color: var(--fg-dim);
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }

  .traffic-stat strong {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size, 12px) * 0.9);
  }

  .traffic-legend {
    display: flex;
    gap: 16px;
    padding-bottom: 6px;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 4px;
    font-weight: 600;
  }
  .legend-item.rx { color: var(--chart-net-rx, var(--green)); }
  .legend-item.tx { color: var(--chart-net-tx, var(--yellow)); }

  .traffic-chart {
    width: 100%;
    height: calc(100% - 32px);
    min-height: 180px;
    border-radius: var(--radius-sm, 4px);
    overflow: hidden;
  }

  .traffic-chart.hidden {
    display: none;
  }

  @media (max-width: 960px) {
    .tab-grid {
      grid-template-columns: 1fr !important;
    }

    .side-resize-divider {
      display: none;
    }

    .tab-side {
      border-left: none;
      border-top: 1px solid var(--border);
    }
  }
</style>
