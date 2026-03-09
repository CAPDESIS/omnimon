<script lang="ts">
  import ContextAiChat from "./ContextAiChat.svelte";
  import ConnectionDetail from "./network/ConnectionDetail.svelte";
  import NetworkAlertConfig from "./NetworkAlertConfig.svelte";
  import Skeleton from "./Skeleton.svelte";
  import { tick } from "svelte";
  import { networkConnections, networkTelemetryStatus } from "../stores/security";
  import { metricsHistory } from "../stores/metricsHistory";
  import { theme } from "../stores/preferences";
  import { fade, slide } from "svelte/transition";
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

  interface Props {
    mode?: "basic" | "pro";
    extraHeight?: number;
    filter?: string;
  }

  let { mode = "pro", extraHeight = 0, filter = "" }: Props = $props();

  let collapsed = $state(false);
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
  let proMode = $derived(mode === "pro");
  let aiChatRef: ReturnType<typeof ContextAiChat> | undefined = $state();
  let selectedNodeId = $state<string | null>(null);
  let selectedConnections = $derived($networkConnections.filter(c => c.remote_addr === selectedNodeId));


  // Group connections by process, then by domain
  interface ProcessNode {
    name: string;
    pid: number;
    domains: DomainNode[];
    totalConns: number;
    bytesRecv: number;
  }

  interface DomainNode {
    hostname: string;
    port: number;
    protocol: string;
    count: number;
    bytesRecv: number;
  }

  let processNodes = $derived.by((): ProcessNode[] => {
    if (collapsed) return [];
    const byProc = new Map<string, { pid: number; bytesRecv: number; domains: Map<string, DomainNode> }>();

    for (const conn of $networkConnections) {
      const key = conn.process_name;
      if (!byProc.has(key)) {
        byProc.set(key, { pid: conn.pid, bytesRecv: 0, domains: new Map() });
      }
      const proc = byProc.get(key)!;
      proc.bytesRecv += conn.bytes_recv;
      
      const domKey = `${conn.remote_addr}:${conn.remote_port}`;
      const existing = proc.domains.get(domKey);
      if (existing) {
        existing.count++;
        existing.bytesRecv += conn.bytes_recv;
      } else {
        proc.domains.set(domKey, {
          hostname: conn.remote_addr,
          port: conn.remote_port,
          protocol: conn.protocol,
          count: 1,
          bytesRecv: conn.bytes_recv,
        });
      }
    }

    return [...byProc.entries()]
      .map(([name, data]) => ({
        name,
        pid: data.pid,
        bytesRecv: data.bytesRecv,
        domains: [...data.domains.values()].sort((a, b) => b.count - a.count),
        totalConns: [...data.domains.values()].reduce((s, d) => s + d.count, 0),
      }))
      .sort((a, b) => b.totalConns - a.totalConns);
  });

  let heavyDownloaders = $derived(processNodes.filter(p => p.bytesRecv > 1024 * 1024 * 50)); // > 50MB
  let networkAlertsEnabled = $state(true);

  let totalConnections = $derived($networkConnections.length);
  let processCount = $derived(new Set($networkConnections.map((conn) => `${conn.process_name}:${conn.pid}`)).size);
  let hasTrafficData = $derived($metricsHistory.length > 0 || $networkTelemetryStatus.totalRxBytesPerSec > 0 || $networkTelemetryStatus.totalTxBytesPerSec > 0);
  let hasAnyNetworkData = $derived(totalConnections > 0 || hasTrafficData);
  let summaryCards = $derived(
    collapsed
      ? []
      : [
          { label: t("network.throughput"), value: formatRate($networkTelemetryStatus.totalRxBytesPerSec + $networkTelemetryStatus.totalTxBytesPerSec) },
          { label: t("network.hosts"), value: String(new Set($networkConnections.map((conn) => conn.remote_addr)).size) },
          { label: t("network.mapReady"), value: String(processCount) },
        ],
  );

  $effect(() => {
    if (!proMode && activeTab !== "map") {
      activeTab = "map";
    }
  });

  // --- Canvas-based connection map ---
  let canvas: HTMLCanvasElement | undefined = $state();

  // Debounce canvas redraws to avoid thrashing on rapid store updates
  let drawRafId = 0;
  $effect(() => {
    if (!canvas || collapsed || processNodes.length === 0 || activeTab !== "map") return;
    cancelAnimationFrame(drawRafId);
    drawRafId = requestAnimationFrame(() => {
      if (canvas && !collapsed && activeTab === "map") {
        drawMap(canvas, processNodes);
      }
    });
  });

  // Cache CSS variables — only refresh when theme changes
  let cachedCssVars: { fg: string; fgDim: string; accent: string; green: string; border: string } | null = null;
  $effect(() => {
    // Re-read CSS vars when theme changes
    void $theme;
    cachedCssVars = null;
  });
  function getCssVars() {
    if (cachedCssVars) return cachedCssVars;
    const getVar = (name: string) =>
      getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    cachedCssVars = {
      fg: getVar("--fg") || "#ededef",
      fgDim: getVar("--fg-dim") || "#71717a",
      accent: getVar("--accent") || "#3b82f6",
      green: getVar("--green") || "#22c55e",
      border: getVar("--border") || "#27272a",
    };
    return cachedCssVars;
  }

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

    const { fg, fgDim, accent, green, border } = getCssVars();

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
    } catch (err) {
      console.warn("[NetworkMap] Chart load failed", err);
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
    if (collapsed || activeTab !== "table") return [];
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

  let visibleConnections = $derived.by(() => {
    const query = filter.trim().toLowerCase();
    const scoped = !query
      ? sortedConnections
      : sortedConnections.filter((conn) =>
          conn.process_name.toLowerCase().includes(query)
          || conn.remote_addr.toLowerCase().includes(query)
          || String(conn.remote_port).includes(query),
        );
    return scoped.slice(0, 100);
  });

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

  function onSideResizeKeydown(event: KeyboardEvent) {
    const step = event.shiftKey ? 32 : 16;
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      sidePanelWidth = Math.min(sidePanelWidth + step, 520);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      sidePanelWidth = Math.max(sidePanelWidth - step, 260);
    }
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

  function onResizeKeydown(event: KeyboardEvent) {
    const step = event.shiftKey ? 40 : 20;
    if (event.key === "ArrowUp") {
      event.preventDefault();
      panelHeight = clampPanelHeight(panelHeight - step);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      panelHeight = clampPanelHeight(panelHeight + step);
    }
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
      system_instruction: "You are an AI translator for non-technical users. Analyze the provided network processes and explain them using very simple layman terms, like 'apples and oranges' (peras y manzanas). For example, if a process is downloading heavily, explain it simply. Answer in Spanish as requested by system rules.",
      prompt: question,
      active_tab: activeTab,
      summary: {
        total_connections: totalConnections,
        visible_processes: processCount,
        panel_height: panelHeight,
        capture_backend: $networkTelemetryStatus.captureBackend,
        using_fallback: $networkTelemetryStatus.usingFallback,
      },
      connections: summarizeConnections($networkConnections),
      heavy_downloaders: heavyDownloaders.map(d => ({ process: d.name, bytes_recv: d.bytesRecv })),
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

  function askAiAboutHost(host: string) {
    const proc = selectedConnections[0]?.process_name || "Desconocido";
    const port = selectedConnections[0]?.remote_port || 0;
    const ip = selectedConnections[0]?.remote_addr || host;
    const question = `El proceso ${proc} tiene una conexión a ${ip}:${port} (${host}). ¿Qué es este servicio? ¿Es seguro? ¿Debería estar ahí?`;
    if (aiChatRef) aiChatRef.ask(question);
  }

  function setActiveTab(tab: "map" | "table" | "traffic") {
    activeTab = tab;
  }

  function sortHeaderLabel(key: typeof tableSortKey): string {
    const direction = tableSortKey === key ? (tableSortAsc ? "ascending" : "descending") : "sortable";
    return `${key} ${direction}`;
  }

  function mapCanvasDescription(): string {
    if (processNodes.length === 0) return t("network.waiting");
    const topNode = processNodes[0];
    return `${t("network.summary", { connections: String(totalConnections), processes: String(processCount) })}. ${topNode.name}: ${topNode.totalConns}.`;
  }

</script>

<div class="netmap-section">
    {#if hasAnyNetworkData}
      <div id="network-map-panel" class="netmap-body" style={`height:${panelHeight + extraHeight}px`}>
        <div class="summary-strip" transition:fade={{ duration: 180 }}>
          {#each summaryCards as card (card.label)}
            <div class="summary-card">
              <span class="summary-label">{card.label}</span>
              <strong class="summary-value">{card.value}</strong>
            </div>
          {/each}
        </div>

        <!-- Tab bar -->
        <div class="tab-bar" role="tablist">
          <button
            class="tab-btn"
            class:active={activeTab === "map"}
            onclick={() => setActiveTab("map")}
            role="tab"
            aria-selected={activeTab === "map"}
            id="network-tab-map"
            aria-controls="network-panel-map"
            tabindex={activeTab === "map" ? 0 : -1}
          >{t("network.map")}</button>
          {#if proMode}
            <button
              class="tab-btn"
              class:active={activeTab === "table"}
              onclick={() => setActiveTab("table")}
              role="tab"
              aria-selected={activeTab === "table"}
              id="network-tab-table"
              aria-controls="network-panel-table"
              tabindex={activeTab === "table" ? 0 : -1}
            >{t("network.connections")}</button>
            <button
              class="tab-btn"
              class:active={activeTab === "traffic"}
              onclick={() => setActiveTab("traffic")}
              role="tab"
              aria-selected={activeTab === "traffic"}
              id="network-tab-traffic"
              aria-controls="network-panel-traffic"
              tabindex={activeTab === "traffic" ? 0 : -1}
            >{t("network.traffic")}</button>
          {/if}
        </div>

        {#if !proMode}
          <div class="basic-banner">{t("network.basicSummary")}</div>
        {/if}

        <div class="tab-grid" style={`grid-template-columns:${proMode ? `minmax(0,1fr) 6px minmax(260px, ${sidePanelWidth}px)` : "minmax(0,1fr)"}`}>
          <div class="tab-main">
            {#if totalConnections === 0 && activeTab !== "traffic"}
              <div class="empty-state">{t("network.waiting")}</div>
            {/if}

            <!-- Map Tab -->
            {#if activeTab === "map"}
              <div class="tab-content map-content" id="network-panel-map" role="tabpanel" aria-labelledby="network-tab-map">
                {#if totalConnections === 0}
                  <div class="empty-state">{t("network.waiting")}</div>
                {:else}
                  <canvas
                    bind:this={canvas}
                    class="netmap-canvas"
                    height={Math.min(processNodes.length * 40 + 20, Math.max(panelHeight - 120, 220))}
                    aria-label={mapCanvasDescription()}
                  ></canvas>
                  <p class="sr-only">{mapCanvasDescription()}</p>
                  <div class="netmap-list">
                    {#each processNodes as node (node.name)}
                      <div class="netmap-proc">
                        <span class="proc-name">{node.name}</span>
                        <span class="proc-count">{node.totalConns}</span>
                        <div class="domain-chips">
                          {#each node.domains.slice(0, 5) as domain}
                            <span class="domain-chip clickable-chip" onclick={() => selectedNodeId = domain.hostname} role="button" tabindex="0" title="{domain.hostname}:{domain.port} ({domain.protocol})">
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
              <div class="tab-content table-content" id="network-panel-table" role="tabpanel" aria-labelledby="network-tab-table">
                <div class="network-help">{t("network.connectionsHelp")}</div>
                {#if totalConnections === 0}
                  <div class="empty-state">{t("network.waiting")}</div>
                {:else}
                  <table class="conn-table" aria-label="Active connections">
                  <thead>
                    <tr class="clickable-row" onclick={() => selectedNodeId = conn.remote_addr}>
                      <th class="sortable" scope="col"><button type="button" class="sort-button" onclick={() => setTableSort("process")} aria-label={sortHeaderLabel("process")}>{t("network.process")}<span aria-hidden="true">{sortArrow("process")}</span></button></th>
                      <th class="sortable" scope="col"><button type="button" class="sort-button" onclick={() => setTableSort("addr")} aria-label={sortHeaderLabel("addr")}>{t("network.destination")}<span aria-hidden="true">{sortArrow("addr")}</span></button></th>
                      <th class="sortable" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => setTableSort("port")} aria-label={sortHeaderLabel("port")}>{t("network.port")}<span aria-hidden="true">{sortArrow("port")}</span></button></th>
                      <th class="sortable" scope="col"><button type="button" class="sort-button" onclick={() => setTableSort("direction")} aria-label={sortHeaderLabel("direction")}>{t("network.direction")}<span aria-hidden="true">{sortArrow("direction")}</span></button></th>
                      <th class="sortable" scope="col"><button type="button" class="sort-button" onclick={() => setTableSort("proto")} aria-label={sortHeaderLabel("proto")}>{t("network.protocol")}<span aria-hidden="true">{sortArrow("proto")}</span></button></th>
                      <th class="sortable" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => setTableSort("bytes")} aria-label={sortHeaderLabel("bytes")}>{t("network.bytes")}<span aria-hidden="true">{sortArrow("bytes")}</span></button></th>
                      <th class="sortable" scope="col"><button type="button" class="sort-button" onclick={() => setTableSort("state")} aria-label={sortHeaderLabel("state")}>{t("network.state")}<span aria-hidden="true">{sortArrow("state")}</span></button></th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each visibleConnections as conn (conn.pid + conn.remote_addr + conn.remote_port + conn.direction)}
                      <tr class="clickable-row" onclick={() => selectedNodeId = conn.remote_addr}>
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
              <div class="tab-content traffic-content" id="network-panel-traffic" role="tabpanel" aria-labelledby="network-tab-traffic">
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
                  <div class="traffic-chart" class:hidden={chartLoadFailed || !hasTrafficData} bind:this={trafficChartEl} role="img" aria-label={t("network.trafficHelp")}></div>
                </div>
            {/if}
          </div>

          {#if selectedNodeId}
            <div class="connection-detail-overlay">
              <ConnectionDetail
                nodeId={selectedNodeId}
                connections={selectedConnections}
                onClose={() => selectedNodeId = null}
                onAskAi={askAiAboutHost}
              />
            </div>
          {/if}
          {#if proMode}
            <button type="button" class="side-resize-divider" class:active={sideDragMode === "sidebar"} onmousedown={startSideResize} onkeydown={onSideResizeKeydown} aria-label={t("common.expand")}></button>
          {/if}

          {#if proMode}
            <div class="tab-side">
              <div class="capture-chip">{t("network.captureBackend", { backend: $networkTelemetryStatus.captureBackend })}</div>
              <div class="capture-chip">RX {$networkTelemetryStatus.totalRxBytesPerSec > 0 ? formatRate($networkTelemetryStatus.totalRxBytesPerSec) : "0 B/s"}</div>
              <div class="capture-chip">TX {$networkTelemetryStatus.totalTxBytesPerSec > 0 ? formatRate($networkTelemetryStatus.totalTxBytesPerSec) : "0 B/s"}</div>

              <div class="network-alerts">
                <label style="display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--fg-dim);">
                  <input type="checkbox" bind:checked={networkAlertsEnabled} />
                  {t("network.enableAlerts") || "Activar Alertas de Descarga"}
                </label>
                {#if networkAlertsEnabled && heavyDownloaders.length > 0}
                  <div class="network-warning" style="margin-top: 6px;">
                    ⚠️ {heavyDownloaders.length} proceso(s) consumiendo mucho ancho de banda (ej. {heavyDownloaders[0].name}).
                  </div>
                {/if}
              </div>

              <div class="network-help">{t("network.mapDeepInfo")}</div>
              {#if $networkTelemetryStatus.usingFallback}
                <div class="network-warning">{t("network.fallbackNotice")}</div>
              {/if}
              <NetworkAlertConfig />
              <ContextAiChat
                bind:this={aiChatRef}
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
          {/if}
        </div>

        <button
          type="button"
          class="netmap-resize-divider"
          class:active={dragMode === "content"}
          onmousedown={startResize}
          onkeydown={onResizeKeydown}
          aria-label={t("common.expand")}
        ></button>
      </div>
    {/if}
</div>



  <style>
.netmap-section {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    background: var(--bg-alt);
  }

  .netmap-body {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .summary-strip {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 10px;
    padding: 12px 10px 0;
  }

  .summary-card {
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-surface, var(--bg-alt)) 92%, white 3%), transparent 140%);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .summary-label {
    color: var(--fg-dim);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-size: calc(var(--base-font-size, 12px) * 0.68);
    font-weight: 700;
  }

  .summary-value {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size, 12px) * 0.95);
  }

  .basic-banner {
    margin: 10px;
    padding: 10px 12px;
    border: 1px solid color-mix(in srgb, var(--border) 82%, transparent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg) 92%, white 3%);
    color: var(--fg-dim);
    line-height: 1.45;
    font-size: calc(var(--base-font-size, 12px) * 0.78);
  }

  .tab-grid {
    display: grid;
    min-height: 0;
    flex: 1;
  }

  .tab-main,
  .tab-side {
    min-height: 0;
    overflow-y: auto;
  }

  .tab-side {
    border-left: 1px solid var(--border);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: rgba(0, 0, 0, 0.06);
    animation: side-enter 180ms ease-out;
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
    padding: 0;
    border: none;
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
    width: 100%;
    padding: 0;
    border: none;
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
    border-radius: 10px;
    transition: background 0.18s ease, transform 0.18s ease;
  }

  .netmap-proc:hover {
    background: color-mix(in srgb, var(--bg-hover) 92%, transparent);
    transform: translateX(2px);
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
  .sort-button {
    width: 100%;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    text-transform: inherit;
    letter-spacing: inherit;
    text-align: left;
    padding: 0;
    cursor: pointer;
  }

  .sort-button-num {
    text-align: right;
  }

  .sort-button:hover,
  .sort-button:focus-visible {
    color: var(--fg);
  }

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
    transition: opacity 0.18s ease, transform 0.18s ease;
  }

  .traffic-chart.hidden {
    display: none;
  }

  @keyframes side-enter {
    from {
      opacity: 0;
      transform: translateX(8px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
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

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }



/* Appended programmatically */
.clickable-chip {
  cursor: pointer;
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.clickable-chip:hover {
  opacity: 0.8;
  transform: translateY(-1px);
}
.connection-detail-overlay {
  position: absolute;
  top: 50px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 100;
  min-width: 300px;
  max-width: 90%;
}



.clickable-row {
  cursor: pointer;
}

.clickable-row {
  cursor: pointer;
}

</style>
