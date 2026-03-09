import { describe, expect, it } from "vitest";

import { detectBrowser } from "../browser";
import type { ProcessEntry } from "../types";

function makeProc(overrides: Partial<ProcessEntry> = {}): ProcessEntry {
  return {
    pid: 1,
    name: "Chrome",
    exec_name: "Google Chrome Helper",
    exe_path: "/Applications/Google Chrome.app",
    bundle_id: null,
    icon_data_url: null,
    ram_mb: 100,
    cpu_pct: 10,
    disk_read_mb: 0,
    disk_write_mb: 0,
    net_rx_bytes_per_sec: 0,
    net_tx_bytes_per_sec: 0,
    energy_impact_score: 0,
    uptime: "1h",
    group: "Browser",
    group_key: "browser:chrome",
    group_identity_type: "normalized_name",
    grouped_name: "Chrome",
    process_count: 1,
    is_system: false,
    idle: false,
    state: "R",
    ...overrides,
  };
}

describe("detectBrowser", () => {
  it("detecta browser por grouped_name", () => {
    expect(detectBrowser(makeProc({ grouped_name: "Chrome" }))).toBe("Chrome");
    expect(detectBrowser(makeProc({ grouped_name: "Safari" }))).toBe("Safari");
    expect(detectBrowser(makeProc({ grouped_name: "Firefox" }))).toBe("Firefox");
  });

  it("detecta browser por exec_name o name cuando grouped_name no alcanza", () => {
    expect(detectBrowser(makeProc({ grouped_name: "Other", exec_name: "Microsoft Edge Helper", name: "Renderer" }))).toBe("Edge");
    expect(detectBrowser(makeProc({ grouped_name: "Other", exec_name: "Unknown", name: "com.apple.WebKit.WebContent" }))).toBe("Safari");
    expect(detectBrowser(makeProc({ grouped_name: "Other", exec_name: "Unknown", name: "firefox" }))).toBe("Firefox");
  });

  it("retorna null para procesos no browser o malformados", () => {
    expect(detectBrowser(makeProc({ group: "System" }))).toBeNull();
    expect(detectBrowser(makeProc({ grouped_name: "Other", exec_name: "daemon", name: "helper" }))).toBeNull();
  });
});
