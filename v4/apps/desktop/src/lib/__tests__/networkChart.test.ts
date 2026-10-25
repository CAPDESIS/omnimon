import { areaPath, donutSegments, linePath, maxRate } from "../networkChart";

const hist = [
  { total_bytes_per_sec_up: 100, total_bytes_per_sec_down: 200 },
  { total_bytes_per_sec_up: 300, total_bytes_per_sec_down: 50 },
];

describe("networkChart", () => {
  it("maxRate is max of up/down, 0 when empty", () => {
    expect(maxRate(hist)).toBe(300);
    expect(maxRate([])).toBe(0);
  });
  it("linePath starts M, scales peak to top padding, empty -> ''", () => {
    const p = linePath([0, 300], 600, 150, 8, 300);
    expect(p.startsWith("M8.0,142.0")).toBe(true);
    expect(p).toContain("592.0,8.0");
    expect(linePath([], 600, 150, 8, 300)).toBe("");
    expect(linePath([5], 600, 150, 8, 0)).toContain("300.0");
  });
  it("areaPath closes to baseline", () => {
    expect(areaPath([100, 200], 600, 150, 8, 200).endsWith("Z")).toBe(true);
    expect(areaPath([], 600, 150, 8, 1)).toBe("");
  });
  it("donutSegments sorts desc, shares vs grand total, offsets accumulate", () => {
    const s = donutSegments([
      { name: "A", totalUp: 100, totalDown: 100 },
      { name: "B", totalUp: 50, totalDown: 50 },
      { name: "Z", totalUp: 0, totalDown: 0 },
    ], 70);
    expect(s.map((x) => x.name)).toEqual(["A", "B"]);
    expect(s[0].share).toBeCloseTo(2 / 3);
    expect(s[1].offset).toBeCloseTo(-(2 / 3) * 2 * Math.PI * 70);
    expect(s[0].dash.split(" ")[0]).toBeCloseTo((2 / 3) * 2 * Math.PI * 70, 1);
  });
  it("donutSegments empty when all zero", () => {
    expect(donutSegments([{ name: "A", totalUp: 0, totalDown: 0 }], 70)).toEqual([]);
  });
});
