<script lang="ts">
  import { onMount } from "svelte";
  import { untrack } from "svelte";
  import { theme } from "../stores/preferences";
  import type { MetricPoint } from "../stores/metricsHistory";
  import {
    createChart,
    AreaSeries,
    ColorType,
    CrosshairMode,
    LineStyle,
    type Time,
    type IChartApi,
    type ISeriesApi,
  } from "lightweight-charts";

  interface SeriesConfig {
    data: MetricPoint[];
    color: string;
    label?: string;
  }

  interface Props {
    series: SeriesConfig[];
    sparkline?: boolean;
    maxY?: number;
    height?: string;
    priceFormat?: "percent" | "decimal" | "bytes" | "megabytes";
  }

  let {
    series,
    sparkline = false,
    maxY,
    height = "180px",
    priceFormat = "decimal",
  }: Props = $props();

  let containerEl: HTMLDivElement | undefined = $state();
  let chartApi: IChartApi | null = null;
  let seriesApis: ISeriesApi<"Area">[] = [];
  let lastColors: string[] = [];
  /** Last synced timestamp per series — used for incremental updates */
  let lastSyncedTime: number[] = [];
  /** Last synced data length per series — used to detect buffer resets */
  let lastSyncedLen: number[] = [];
  /** Last applied maxY — used to detect dynamic scale changes */
  let lastMaxY: number | undefined;
  let alive = false;
  let knownTheme: string | undefined;

  function getVar(name: string): string {
    if (typeof document === "undefined") return "";
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }

  function resolveColor(colorRef: string): string {
    if (colorRef.startsWith("--")) return getVar(colorRef) || "#3b82f6";
    return colorRef;
  }

  function hexAlpha(hex: string, alpha: string): string {
    return hex + alpha;
  }

  function fmtBytes(v: number): string {
    if (v >= 1_048_576) return (v / 1_048_576).toFixed(1) + " MB/s";
    if (v >= 1024) return (v / 1024).toFixed(1) + " KB/s";
    return v.toFixed(0) + " B/s";
  }

  function fmtPercent(v: number): string {
    return v.toFixed(1) + "%";
  }

  function fmtMegabytes(v: number): string {
    if (v >= 1024) return (v / 1024).toFixed(1) + " GB";
    return v.toFixed(0) + " MB";
  }

  function buildPriceFormat() {
    if (priceFormat === "percent") {
      return {
        type: "custom" as const,
        minMove: 0.1,
        formatter: fmtPercent,
        tickmarksFormatter: (vals: number[]) => vals.map(fmtPercent),
      };
    }
    if (priceFormat === "bytes") {
      return {
        type: "custom" as const,
        minMove: 1,
        formatter: fmtBytes,
        tickmarksFormatter: (vals: number[]) => vals.map(fmtBytes),
      };
    }
    if (priceFormat === "megabytes") {
      return {
        type: "custom" as const,
        minMove: 1,
        formatter: fmtMegabytes,
        tickmarksFormatter: (vals: number[]) => vals.map(fmtMegabytes),
      };
    }
    return {
      type: "custom" as const,
      minMove: 0.1,
      formatter: (v: number) => v.toFixed(1),
      tickmarksFormatter: (vals: number[]) => vals.map((v) => v.toFixed(1)),
    };
  }

  function initChart(container: HTMLDivElement) {
    if (chartApi) return;
    // Container must have real dimensions before chart can render
    if (container.clientWidth < 4 || container.clientHeight < 4) {
      console.warn("[TvChart] Container too small:", container.clientWidth, "x", container.clientHeight);
      return;
    }

    try {
      const bg = getVar("--chart-bg") || getVar("--bg-primary") || "#0a0a0b";
      const fg = getVar("--fg-dim") || "#a1a1aa";
      const grid = getVar("--chart-grid") || "#141418";
      const border = getVar("--border") || "#27272a";

      const chart = createChart(container, {
        autoSize: true,
        layout: {
          background: { type: ColorType.Solid, color: sparkline ? "transparent" : bg },
          textColor: fg,
          fontSize: sparkline ? 9 : 11,
          fontFamily: "'SF Mono', 'Menlo', 'Consolas', monospace",
          attributionLogo: !sparkline,
        },
        grid: {
          vertLines: { visible: !sparkline, color: grid },
          horzLines: { visible: !sparkline, color: grid },
        },
        timeScale: {
          visible: !sparkline,
          timeVisible: true,
          secondsVisible: false,
          borderColor: border,
          fixLeftEdge: true,
          fixRightEdge: true,
        },
        rightPriceScale: {
          visible: !sparkline,
          borderColor: border,
          scaleMargins: sparkline ? { top: 0.05, bottom: 0.05 } : { top: 0.1, bottom: 0.1 },
        },
        crosshair: sparkline
          ? { mode: CrosshairMode.Hidden }
          : {
              mode: CrosshairMode.Normal,
              vertLine: { color: border, width: 1, style: LineStyle.Dashed },
              horzLine: { color: border, width: 1, style: LineStyle.Dashed },
            },
        handleScroll: !sparkline,
        handleScale: !sparkline,
      });

      chartApi = chart;
      seriesApis = [];
      lastColors = [];
      lastSyncedTime = [];
      lastSyncedLen = [];

      const currentSeries = untrack(() => series);

      lastMaxY = maxY;

      for (const s of currentSeries) {
        const color = resolveColor(s.color);

        const areaSeries = chart.addSeries(AreaSeries, {
          lineColor: color,
          topColor: hexAlpha(color, sparkline ? "30" : "40"),
          bottomColor: hexAlpha(color, "05"),
          lineWidth: sparkline ? 1.5 : 2,
          crosshairMarkerVisible: !sparkline,
          priceLineVisible: false,
          lastValueVisible: !sparkline,
          title: sparkline ? "" : (s.label ?? ""),
          priceFormat: buildPriceFormat(),
          ...(maxY != null
            ? { autoscaleInfoProvider: () => ({ priceRange: { minValue: 0, maxValue: maxY } }) }
            : {}),
        });

        const points = s.data.map((p) => ({ time: p.time as Time, value: p.value }));
        areaSeries.setData(points);
        seriesApis.push(areaSeries);
        lastColors.push(color);
        const lastPt = s.data[s.data.length - 1];
        lastSyncedTime.push(lastPt ? lastPt.time : 0);
        lastSyncedLen.push(s.data.length);
      }

      chart.timeScale().fitContent();
      console.debug("[TvChart] Created OK. Series:", seriesApis.length, "Size:", container.clientWidth, "x", container.clientHeight);
    } catch (err) {
      console.error("[TvChart] Init failed:", err);
    }
  }

  function syncData() {
    // Retry init if chart was not created (container may not have had dimensions)
    if (!chartApi && containerEl) {
      initChart(containerEl);
      if (!chartApi) return;
    }
    if (!chartApi || seriesApis.length === 0) return;

    // Update autoscale range if maxY changed dynamically
    if (maxY !== lastMaxY) {
      lastMaxY = maxY;
      for (const api of seriesApis) {
        api.applyOptions({
          autoscaleInfoProvider: maxY != null
            ? () => ({ priceRange: { minValue: 0, maxValue: maxY } })
            : undefined,
        });
      }
    }

    for (let i = 0; i < series.length && i < seriesApis.length; i++) {
      const s = series[i];
      const data = s.data;
      if (data.length === 0) continue;

      const prevTime = lastSyncedTime[i] ?? 0;
      const prevLen = lastSyncedLen[i] ?? 0;
      const lastPt = data[data.length - 1];
      const firstPt = data[0];

      // Detect buffer reset/trim: if current data is shorter than what we
      // synced before, or if the first point's time jumped forward (the
      // rolling buffer wrapped and older entries were dropped beyond what
      // incremental update can handle), fall back to full setData().
      const bufferReset = data.length < prevLen - 1 || (prevLen > 0 && firstPt.time > prevTime);

      if (bufferReset || prevTime === 0) {
        // Full setData — only on first sync or buffer reset
        const points = data.map((p) => ({ time: p.time as Time, value: p.value }));
        seriesApis[i].setData(points);
      } else if (lastPt.time > prevTime) {
        // Incremental: only push genuinely new data points
        for (let j = data.length - 1; j >= 0; j--) {
          if (data[j].time <= prevTime) {
            // Push all points after index j
            for (let k = j + 1; k < data.length; k++) {
              seriesApis[i].update({ time: data[k].time as Time, value: data[k].value });
            }
            break;
          }
          if (j === 0) {
            // All points are new (shouldn't normally happen, but be safe)
            for (let k = 0; k < data.length; k++) {
              seriesApis[i].update({ time: data[k].time as Time, value: data[k].value });
            }
          }
        }
      }
      // else: lastPt.time === prevTime — no new data, skip entirely

      lastSyncedTime[i] = lastPt.time;
      lastSyncedLen[i] = data.length;

      // Update color if changed (for dynamic color sparklines)
      const newColor = resolveColor(s.color);
      if (newColor !== lastColors[i]) {
        lastColors[i] = newColor;
        seriesApis[i].applyOptions({
          lineColor: newColor,
          topColor: hexAlpha(newColor, sparkline ? "30" : "40"),
          bottomColor: hexAlpha(newColor, "05"),
        });
      }
    }
    // NOTE: no fitContent() here — avoids layout thrashing on every poll cycle
  }

  function destroyChart() {
    if (chartApi) {
      chartApi.remove();
      chartApi = null;
      seriesApis = [];
      lastColors = [];
      lastSyncedTime = [];
      lastSyncedLen = [];
    }
  }

  onMount(() => {
    alive = true;
    knownTheme = untrack(() => $theme);
    // RAF ensures the container has been laid out with real dimensions
    requestAnimationFrame(() => {
      if (alive && containerEl) initChart(containerEl);
    });
    return () => {
      alive = false;
      destroyChart();
    };
  });

  // Recreate chart ONLY when theme actually changes (skip initial run)
  $effect(() => {
    const currentTheme = $theme;
    untrack(() => {
      if (!alive || !containerEl) return;
      if (currentTheme === knownTheme) return;
      knownTheme = currentTheme;
      destroyChart();
      requestAnimationFrame(() => {
        if (alive && containerEl) initChart(containerEl);
      });
    });
  });

  // Sync data on every store update — lightweight dep check without allocating arrays
  $effect(() => {
    // Touch only the last timestamp and length of each series + color to track deps.
    // This avoids creating a new array via .map() on every trigger.
    let _dep = 0;
    for (let i = 0; i < series.length; i++) {
      const s = series[i];
      const d = s.data;
      _dep += (d[d.length - 1]?.time ?? 0) + d.length;
      // Access color to track color changes
      void s.color;
    }
    // Track maxY changes for dynamic scaling
    void maxY;
    untrack(() => syncData());
  });
</script>

<div
  class="tv-chart"
  class:tv-chart--sparkline={sparkline}
  style={sparkline ? "" : `height:${height}`}
  bind:this={containerEl}
></div>

<style>
  .tv-chart {
    width: 100%;
    border-radius: var(--radius-md, 8px);
    overflow: hidden;
  }

  .tv-chart--sparkline {
    height: 100%;
  }
</style>
