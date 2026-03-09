import type { MetricPoint } from "../stores/metricsHistory";

export type MetricKind = "cpu" | "ram" | "network" | "swap" | "processes";
export type SortKey = "name" | "pid" | "cpu" | "ram" | "net" | "state" | "uptime";

export function defaultSortKey(kind: MetricKind): SortKey {
  if (kind === "cpu") return "cpu";
  if (kind === "ram" || kind === "swap") return "ram";
  if (kind === "network") return "net";
  return "ram";
}

function formatSeries(series: MetricPoint[], suffix = ""): string {
  if (series.length === 0) return "\u2014";
  const last = series[series.length - 1]?.value ?? 0;
  const max = Math.max(...series.map((point) => point.value), 0);
  const avg = series.reduce((sum, point) => sum + point.value, 0) / Math.max(series.length, 1);
  return `Now ${last.toFixed(1)}${suffix} · Avg ${avg.toFixed(1)}${suffix} · Max ${max.toFixed(1)}${suffix}`;
}

export function getSparklineColor(metric: string, series: Array<{ value: number }>): string {
  if (metric !== "cpu" && metric !== "ram") return "var(--accent)";
  if (series.length === 0) return "var(--accent)";
  const latest = series[series.length - 1].value;
  if (latest >= 80) return "var(--danger)";
  if (latest >= 60) return "var(--yellow)";
  return "var(--green)";
}

export function sparklinePath(series: MetricPoint[], width = 200, height = 32): string {
  if (series.length < 2) return "";
  const max = Math.max(...series.map((point) => point.value), 1);
  const step = width / (series.length - 1);
  return series
    .map((point, index) => {
      const x = index * step;
      const y = height - (point.value / max) * height;
      return `${index === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

export function activeSeriesForMetric(
  metric: MetricKind,
  series: {
    cpuSeries: MetricPoint[];
    ramSeries: MetricPoint[];
    swapSeries: MetricPoint[];
  },
): MetricPoint[] {
  switch (metric) {
    case "cpu":
      return series.cpuSeries;
    case "ram":
      return series.ramSeries;
    case "swap":
      return series.swapSeries;
    default:
      return [];
  }
}

export function metricSummaryLabel(
  metric: MetricKind,
  values: {
    cpuSeries: MetricPoint[];
    ramSeries: MetricPoint[];
    swapSeries: MetricPoint[];
    totalProcesses?: number;
  },
): string {
  if (metric === "cpu") return formatSeries(values.cpuSeries, "%");
  if (metric === "ram") return formatSeries(values.ramSeries, "%");
  if (metric === "swap") return formatSeries(values.swapSeries, " MB");
  return `${values.totalProcesses ?? 0} visible`;
}

export function loadNetworkMap(): Promise<unknown> {
  return import("../components/NetworkMap.svelte");
}
