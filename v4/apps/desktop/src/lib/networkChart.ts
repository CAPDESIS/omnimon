/** Pure SVG geometry for network charts (no chart library). */

export interface HistoryLike {
  total_bytes_per_sec_up: number;
  total_bytes_per_sec_down: number;
}

export function maxRate(history: HistoryLike[]): number {
  let max = 0;
  for (const s of history) max = Math.max(max, s.total_bytes_per_sec_up, s.total_bytes_per_sec_down);
  return max;
}

function pts(values: number[], w: number, h: number, pad: number, max: number): [number, number][] {
  const iw = Math.max(1, w - pad * 2);
  const ih = Math.max(1, h - pad * 2);
  const n = values.length;
  return values.map((v, i) => [
    pad + (n <= 1 ? iw / 2 : (i / (n - 1)) * iw),
    pad + ih - (max > 0 ? (v / max) * ih : 0),
  ]);
}

export function linePath(values: number[], w: number, h: number, pad: number, max: number): string {
  if (values.length === 0) return "";
  return pts(values, w, h, pad, max).map(([x, y], i) => `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`).join(" ");
}

export function areaPath(values: number[], w: number, h: number, pad: number, max: number): string {
  if (values.length === 0) return "";
  const p = pts(values, w, h, pad, max);
  const base = (h - pad).toFixed(1);
  return `${linePath(values, w, h, pad, max)} L${p[p.length - 1][0].toFixed(1)},${base} L${p[0][0].toFixed(1)},${base} Z`;
}

export const DONUT_COLORS = ["#3b82f6", "#22c55e", "#f59e0b", "#ef4444", "#8b5cf6", "#06b6d4", "#f97316", "#64748b"];

export interface DonutSegment { name: string; total: number; share: number; dash: string; offset: number; color: string }

/** Top-`limit` processes by total rate as donut segments; shares are vs. the grand total. */
export function donutSegments(
  items: Array<{ name: string; totalUp: number; totalDown: number }>,
  radius: number,
  limit = 8,
): DonutSegment[] {
  const totals = items.map((i) => ({ name: i.name, total: i.totalUp + i.totalDown })).filter((i) => i.total > 0)
    .sort((a, b) => b.total - a.total);
  const grand = totals.reduce((s, i) => s + i.total, 0);
  if (grand <= 0) return [];
  const c = 2 * Math.PI * radius;
  let acc = 0;
  return totals.slice(0, limit).map((i, idx) => {
    const share = i.total / grand;
    const seg = { name: i.name, total: i.total, share, dash: `${(share * c).toFixed(2)} ${c.toFixed(2)}`, offset: -acc * c, color: DONUT_COLORS[idx % DONUT_COLORS.length] };
    acc += share;
    return seg;
  });
}
