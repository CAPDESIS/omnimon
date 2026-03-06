<script lang="ts">
  import { onMount } from "svelte";
  import { networkConnections } from "../stores/security";
  import { metricsHistory } from "../stores/metricsHistory";
  import { theme } from "../stores/preferences";
  import { slide } from "svelte/transition";
  import type { NetworkConnection } from "../lib/types";

  let collapsed = $state(true);
  let activeTab = $state<"map" | "table" | "traffic">("map");

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

  // --- Canvas-based connection map ---
  let canvas: HTMLCanvasElement | undefined = $state();

  $effect(() => {
    if (!canvas || collapsed || processNodes.length === 0 || activeTab !== "map") return;
    drawMap(canvas, processNodes);
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

    const leftMargin = 120;
    const rightMargin = w - 140;
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
  let chartInstance: unknown = $state(undefined);

  $effect(() => {
    if (!trafficChartEl || collapsed || activeTab !== "traffic") return;
    initTrafficChart(trafficChartEl);
  });

  async function initTrafficChart(container: HTMLDivElement) {
    try {
      const lc = await import("lightweight-charts");
      if (chartInstance) return;

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

      // Populate with history
      const history = $metricsHistory;
      if (history.length > 0) {
        const now = Math.floor(Date.now() / 1000);
        const rxData = history.map((h, i) => ({
          time: (now - (history.length - 1 - i) * 2) as any,
          value: h.netRx / 1024,
        }));
        const txData = history.map((h, i) => ({
          time: (now - (history.length - 1 - i) * 2) as any,
          value: h.netTx / 1024,
        }));
        rxSeries.setData(rxData);
        txSeries.setData(txData);
      }

      chart.timeScale().fitContent();
      chartInstance = chart;

      const ro = new ResizeObserver(() => {
        chart.applyOptions({ width: container.clientWidth });
      });
      ro.observe(container);
    } catch {
      // lightweight-charts not available, show fallback
    }
  }

  // Cleanup chart on collapse
  $effect(() => {
    if (collapsed && chartInstance) {
      try { (chartInstance as any).remove(); } catch {}
      chartInstance = undefined;
    }
  });

  // Recreate chart when theme changes
  $effect(() => {
    const _ = $theme; // subscribe to theme changes
    if (!trafficChartEl || collapsed || activeTab !== "traffic") return;
    if (chartInstance) {
      try { (chartInstance as any).remove(); } catch {}
      chartInstance = undefined;
    }
    // defer to allow CSS vars to update
    requestAnimationFrame(() => {
      if (trafficChartEl && !collapsed && activeTab === "traffic") {
        initTrafficChart(trafficChartEl);
      }
    });
  });

  // --- Connections table sort ---
  let tableSortKey = $state<"process" | "addr" | "port" | "proto" | "state">("process");
  let tableSortAsc = $state(true);

  let sortedConnections = $derived.by(() => {
    const conns = [...$networkConnections];
    conns.sort((a, b) => {
      let va: string | number, vb: string | number;
      switch (tableSortKey) {
        case "process": va = a.process_name; vb = b.process_name; break;
        case "addr": va = a.remote_addr; vb = b.remote_addr; break;
        case "port": va = a.remote_port; vb = b.remote_port; break;
        case "proto": va = a.protocol; vb = b.protocol; break;
        case "state": va = a.state; vb = b.state; break;
      }
      if (typeof va === "string" && typeof vb === "string") {
        return tableSortAsc ? va.localeCompare(vb) : vb.localeCompare(va);
      }
      return tableSortAsc ? Number(va) - Number(vb) : Number(vb) - Number(va);
    });
    return conns;
  });

  function setTableSort(key: typeof tableSortKey) {
    if (tableSortKey === key) tableSortAsc = !tableSortAsc;
    else { tableSortKey = key; tableSortAsc = true; }
  }

  function sortArrow(key: typeof tableSortKey): string {
    if (tableSortKey !== key) return "";
    return tableSortAsc ? " \u25B2" : " \u25BC";
  }
</script>

{#if totalConnections > 0}
  <div class="netmap-section">
    <button
      class="netmap-toggle"
      onclick={() => collapsed = !collapsed}
    >
      <span class="chevron" class:open={!collapsed}>&#9654;</span>
      <span class="netmap-title">Network Map</span>
      <span class="netmap-count">{totalConnections} connections / {processNodes.length} processes</span>
    </button>

    {#if !collapsed}
      <div class="netmap-body" transition:slide={{ duration: 200 }}>
        <!-- Tab bar -->
        <div class="tab-bar" role="tablist">
          <button
            class="tab-btn"
            class:active={activeTab === "map"}
            onclick={() => activeTab = "map"}
            role="tab"
            aria-selected={activeTab === "map"}
          >Map</button>
          <button
            class="tab-btn"
            class:active={activeTab === "table"}
            onclick={() => activeTab = "table"}
            role="tab"
            aria-selected={activeTab === "table"}
          >Connections</button>
          <button
            class="tab-btn"
            class:active={activeTab === "traffic"}
            onclick={() => activeTab = "traffic"}
            role="tab"
            aria-selected={activeTab === "traffic"}
          >Traffic</button>
        </div>

        <!-- Map Tab -->
        {#if activeTab === "map"}
          <div class="tab-content map-content">
            <canvas
              bind:this={canvas}
              class="netmap-canvas"
              height={Math.min(processNodes.length * 40 + 20, 300)}
            ></canvas>
            <div class="netmap-list">
              {#each processNodes as node (node.name)}
                <div class="netmap-proc">
                  <span class="proc-name">{node.name}</span>
                  <span class="proc-count">{node.totalConns}</span>
                  <div class="domain-chips">
                    {#each node.domains.slice(0, 5) as domain}
                      <span class="domain-chip" title="{domain.hostname}:{domain.port} ({domain.protocol})">
                        {domain.hostname}
                      </span>
                    {/each}
                    {#if node.domains.length > 5}
                      <span class="domain-more">+{node.domains.length - 5}</span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Connections Table Tab -->
        {#if activeTab === "table"}
          <div class="tab-content table-content">
            <table class="conn-table" aria-label="Active connections">
              <thead>
                <tr>
                  <th class="sortable" onclick={() => setTableSort("process")}>Process{sortArrow("process")}</th>
                  <th class="sortable" onclick={() => setTableSort("addr")}>Destination{sortArrow("addr")}</th>
                  <th class="sortable" onclick={() => setTableSort("port")}>Port{sortArrow("port")}</th>
                  <th class="sortable" onclick={() => setTableSort("proto")}>Protocol{sortArrow("proto")}</th>
                  <th class="sortable" onclick={() => setTableSort("state")}>State{sortArrow("state")}</th>
                </tr>
              </thead>
              <tbody>
                {#each sortedConnections.slice(0, 50) as conn (conn.pid + conn.remote_addr + conn.remote_port)}
                  <tr>
                    <td class="col-process">{conn.process_name}</td>
                    <td class="col-addr mono">{conn.remote_addr}</td>
                    <td class="col-port mono">{conn.remote_port}</td>
                    <td class="col-proto mono">{conn.protocol.toUpperCase()}</td>
                    <td class="col-state mono">{conn.state}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
            {#if sortedConnections.length > 50}
              <div class="table-overflow">Showing 50 of {sortedConnections.length} connections</div>
            {/if}
          </div>
        {/if}

        <!-- Traffic Chart Tab -->
        {#if activeTab === "traffic"}
          <div class="tab-content traffic-content">
            <div class="traffic-legend">
              <span class="legend-item rx">&#9660; Inbound (KB/s)</span>
              <span class="legend-item tx">&#9650; Outbound (KB/s)</span>
            </div>
            <div class="traffic-chart" bind:this={trafficChartEl}></div>
          </div>
        {/if}
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
    width: 100%;
    padding: 6px 10px;
    border: none;
    background: transparent;
    color: var(--fg);
    cursor: pointer;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    text-align: left;
  }
  .netmap-toggle:hover { background: var(--bg-hover); }

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
    max-height: 400px;
    overflow: hidden;
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
  }

  /* Map content */
  .map-content {
    display: flex;
    gap: 0;
    max-height: 340px;
  }

  .netmap-canvas {
    flex: 1;
    min-width: 0;
    max-height: 300px;
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
    max-height: 340px;
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
  .col-proto { width: 50px; text-align: center; }
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
    max-height: 240px;
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
    height: 180px;
    border-radius: var(--radius-sm, 4px);
    overflow: hidden;
  }
</style>
