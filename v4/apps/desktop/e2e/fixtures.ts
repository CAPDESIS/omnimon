import { expect, test as base, type Page } from "@playwright/test";

import type { BrowserTab, Metrics, NetworkData } from "../src/lib/types";

const MOCK_METRICS: Metrics = {
  stats: {
    cpu_usage_pct: 12.5,
    ram_total_gb: 32,
    ram_used_pct: 58,
    swap_used_mb: 512,
    total_processes: 7,
    net_rx_bytes_per_sec: 1_048_576,
    net_tx_bytes_per_sec: 524_288,
  },
  processes: [
    {
      pid: 101,
      name: "Chrome Helper",
      exec_name: "Google Chrome Helper",
      exe_path: "/Applications/Google Chrome.app",
      bundle_id: "com.google.Chrome",
      icon_data_url: null,
      ram_mb: 612,
      cpu_pct: 14.2,
      disk_read_mb: 12.4,
      disk_write_mb: 2.1,
      net_rx_bytes_per_sec: 640_000,
      net_tx_bytes_per_sec: 240_000,
      energy_impact_score: 7.8,
      uptime: "01:22:10",
      group: "Browser",
      group_key: "chrome",
      group_identity_type: "browser",
      grouped_name: "Chrome",
      process_count: 2,
      is_system: false,
      idle: false,
      state: "Running",
    },
    {
      pid: 202,
      name: "Safari",
      exec_name: "Safari",
      exe_path: "/Applications/Safari.app",
      bundle_id: "com.apple.Safari",
      icon_data_url: null,
      ram_mb: 284,
      cpu_pct: 5.6,
      disk_read_mb: 3.7,
      disk_write_mb: 0.7,
      net_rx_bytes_per_sec: 180_000,
      net_tx_bytes_per_sec: 64_000,
      energy_impact_score: 3.4,
      uptime: "00:43:12",
      group: "Browser",
      group_key: "safari",
      group_identity_type: "browser",
      grouped_name: "Safari",
      process_count: 1,
      is_system: false,
      idle: false,
      state: "Running",
    },
    {
      pid: 303,
      name: "node",
      exec_name: "node",
      exe_path: "/usr/local/bin/node",
      bundle_id: null,
      icon_data_url: null,
      ram_mb: 356,
      cpu_pct: 24.8,
      disk_read_mb: 4.5,
      disk_write_mb: 1.9,
      net_rx_bytes_per_sec: 24_000,
      net_tx_bytes_per_sec: 18_000,
      energy_impact_score: 5.1,
      uptime: "03:14:55",
      group: "Developer Tools",
      group_key: "node",
      group_identity_type: "process",
      grouped_name: "Node",
      process_count: 1,
      is_system: false,
      idle: false,
      state: "Running",
    },
    {
      pid: 404,
      name: "Slack",
      exec_name: "Slack",
      exe_path: "/Applications/Slack.app",
      bundle_id: "com.tinyspeck.slackmacgap",
      icon_data_url: null,
      ram_mb: 228,
      cpu_pct: 3.1,
      disk_read_mb: 1.8,
      disk_write_mb: 0.3,
      net_rx_bytes_per_sec: 12_000,
      net_tx_bytes_per_sec: 7_200,
      energy_impact_score: 2.2,
      uptime: "09:17:03",
      group: "Communication",
      group_key: "slack",
      group_identity_type: "process",
      grouped_name: "Slack",
      process_count: 1,
      is_system: false,
      idle: true,
      state: "Idle",
    },
    {
      pid: 505,
      name: "Figma",
      exec_name: "Figma",
      exe_path: "/Applications/Figma.app",
      bundle_id: "com.figma.Desktop",
      icon_data_url: null,
      ram_mb: 840,
      cpu_pct: 18.3,
      disk_read_mb: 6.2,
      disk_write_mb: 1.1,
      net_rx_bytes_per_sec: 320_000,
      net_tx_bytes_per_sec: 88_000,
      energy_impact_score: 6.3,
      uptime: "02:05:44",
      group: "Design",
      group_key: "figma",
      group_identity_type: "process",
      grouped_name: "Figma",
      process_count: 1,
      is_system: false,
      idle: false,
      state: "Running",
    },
    {
      pid: 606,
      name: "Terminal",
      exec_name: "Terminal",
      exe_path: "/System/Applications/Utilities/Terminal.app",
      bundle_id: "com.apple.Terminal",
      icon_data_url: null,
      ram_mb: 92,
      cpu_pct: 1.7,
      disk_read_mb: 0.4,
      disk_write_mb: 0.1,
      net_rx_bytes_per_sec: 0,
      net_tx_bytes_per_sec: 0,
      energy_impact_score: 1.1,
      uptime: "12:18:21",
      group: "Utilities",
      group_key: "terminal",
      group_identity_type: "process",
      grouped_name: "Terminal",
      process_count: 1,
      is_system: false,
      idle: true,
      state: "Idle",
    },
    {
      pid: 707,
      name: "powershell",
      exec_name: "powershell",
      exe_path: "C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe",
      bundle_id: null,
      icon_data_url: null,
      ram_mb: 144,
      cpu_pct: 9.4,
      disk_read_mb: 2.8,
      disk_write_mb: 0.9,
      net_rx_bytes_per_sec: 44_000,
      net_tx_bytes_per_sec: 20_000,
      energy_impact_score: 4.8,
      uptime: "00:18:35",
      group: "Shell",
      group_key: "powershell",
      group_identity_type: "process",
      grouped_name: "PowerShell",
      process_count: 1,
      is_system: false,
      idle: false,
      state: "Running",
    },
  ],
};

const MOCK_BROWSER_TABS: BrowserTab[] = [
  {
    id: "chrome-1",
    title: "OmniMon Docs",
    url: "https://docs.omnimon.app/getting-started",
    browser: "Chrome",
  },
  {
    id: "chrome-2",
    title: "Playwright Testing",
    url: "https://playwright.dev/docs/intro",
    browser: "Chrome",
  },
  {
    id: "safari-1",
    title: "System Monitoring Notes",
    url: "https://example.com/system-monitoring",
    browser: "Safari",
  },
];

const MOCK_NETWORK_DATA: NetworkData = {
  top_processes: [
    {
      pid: 101,
      process_name: "Chrome Helper",
      rx_bytes_per_sec: 640_000,
      tx_bytes_per_sec: 240_000,
      tcp_packets_per_sec: 160,
      udp_packets_per_sec: 18,
    },
    {
      pid: 303,
      process_name: "node",
      rx_bytes_per_sec: 24_000,
      tx_bytes_per_sec: 18_000,
      tcp_packets_per_sec: 32,
      udp_packets_per_sec: 0,
    },
  ],
  recent_connections: [
    {
      pid: 101,
      protocol: "Tcp",
      direction: "Outbound",
      src_ip: "192.168.1.20",
      dst_ip: "104.26.4.172",
      src_port: 53124,
      dst_port: 443,
      bytes: 980_000,
    },
    {
      pid: 303,
      protocol: "Tcp",
      direction: "Outbound",
      src_ip: "192.168.1.20",
      dst_ip: "140.82.121.4",
      src_port: 53125,
      dst_port: 443,
      bytes: 48_000,
    },
    {
      pid: 707,
      protocol: "Tcp",
      direction: "Outbound",
      src_ip: "192.168.1.20",
      dst_ip: "198.51.100.42",
      src_port: 53126,
      dst_port: 4444,
      bytes: 12_000,
    },
  ],
  net_rx_bytes_per_sec: 1_048_576,
  net_tx_bytes_per_sec: 524_288,
  capture_backend: "mock-capture",
  dpi_active: true,
};

const INITIAL_STORE = {
  activeProfilePreset: "general",
  localePreference: "en",
  profilePreset: "power",
  theme: "dark",
  userMode: "pro",
};

async function installTauriMocks(page: Page) {
  await page.addInitScript(
    ({ browserTabs, chatError, initialStore, metrics, networkData }) => {
      const callbackRegistry = new Map<number, (payload: unknown) => void>();
      const eventListeners = new Map<number, { event: string; handler: number }>();
      const stores = new Map<number, Map<string, unknown>>();
      let nextCallbackId = 1;
      let nextEventId = 1;
      let nextStoreRid = 1;
      let tabsState = browserTabs.map((tab) => ({ ...tab }));

      const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));

      function ensureStore(rid: number): Map<string, unknown> {
        const store = stores.get(rid);
        if (!store) {
          throw new Error(`Unknown mock store ${rid}`);
        }
        return store;
      }

      async function invoke(cmd: string, args: Record<string, unknown> = {}) {
        switch (cmd) {
          case "get_metrics":
            return clone(metrics);
          case "get_browser_tabs":
            return clone(tabsState);
          case "get_network_data":
            return clone(networkData);
          case "kill_process":
            return true;
          case "kill_processes":
            return { killed: clone(Array.isArray(args.pids) ? args.pids : []), failed: [] };
          case "close_browser_tab": {
            const tabId = typeof args.tabId === "string" ? args.tabId : "";
            tabsState = tabsState.filter((tab) => tab.id !== tabId);
            return true;
          }
          case "focus_browser_tab":
            return true;
          case "ai_chat":
            throw new Error(chatError);
          case "analyze_processes":
          case "analyze_context":
            throw new Error(chatError);
          case "save_ai_config":
          case "clear_ai_cache":
            return undefined;
          case "get_cloud_key":
            return "";
          case "save_cloud_key":
            return undefined;
          case "check_api_key":
          case "validate_api_key":
            return false;
          case "get_window_visible":
            return true;
          case "set_network_alert_rules":
            return 1;
          case "list_plugins":
            return [];
          case "plugin:event|listen": {
            const eventId = nextEventId++;
            eventListeners.set(eventId, {
              event: String(args.event ?? "unknown"),
              handler: Number(args.handler ?? 0),
            });
            return eventId;
          }
          case "plugin:event|unlisten":
          case "plugin:event|emit":
          case "plugin:event|emit_to":
            return undefined;
          case "plugin:autostart|is_enabled":
            return false;
          case "plugin:autostart|enable":
          case "plugin:autostart|disable":
            return undefined;
          case "plugin:store|load": {
            const rid = nextStoreRid++;
            stores.set(rid, new Map(Object.entries(initialStore)));
            return rid;
          }
          case "plugin:store|get_store":
            return null;
          case "plugin:store|get": {
            const store = ensureStore(Number(args.rid));
            const key = String(args.key ?? "");
            const exists = store.has(key);
            return [exists ? clone(store.get(key)) : null, exists];
          }
          case "plugin:store|set": {
            const store = ensureStore(Number(args.rid));
            store.set(String(args.key ?? ""), clone(args.value));
            return undefined;
          }
          case "plugin:store|delete": {
            const store = ensureStore(Number(args.rid));
            store.delete(String(args.key ?? ""));
            return undefined;
          }
          case "plugin:store|clear": {
            ensureStore(Number(args.rid)).clear();
            return undefined;
          }
          case "plugin:store|reset": {
            stores.set(Number(args.rid), new Map(Object.entries(initialStore)));
            return undefined;
          }
          case "plugin:store|keys":
            return [...ensureStore(Number(args.rid)).keys()];
          case "plugin:store|values":
            return [...ensureStore(Number(args.rid)).values()].map((value) => clone(value));
          case "plugin:store|entries":
            return [...ensureStore(Number(args.rid)).entries()].map(([key, value]) => [key, clone(value)]);
          case "plugin:store|length":
            return ensureStore(Number(args.rid)).size;
          case "plugin:store|reload":
          case "plugin:store|save":
          case "plugin:resources|close":
            return undefined;
          default:
            return undefined;
        }
      }

      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        configurable: true,
        value: {
          invoke,
          transformCallback(callback: (payload: unknown) => void) {
            const id = nextCallbackId++;
            callbackRegistry.set(id, callback);
            return id;
          },
          unregisterCallback(id: number) {
            callbackRegistry.delete(id);
          },
          convertFileSrc(path: string) {
            return path;
          },
        },
      });

      Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
        configurable: true,
        value: {
          unregisterListener(_event: string, eventId: number) {
            eventListeners.delete(eventId);
          },
        },
      });

      Object.defineProperty(window, "isTauri", {
        configurable: true,
        value: true,
      });
    },
    {
      metrics: MOCK_METRICS,
      browserTabs: MOCK_BROWSER_TABS,
      networkData: MOCK_NETWORK_DATA,
      initialStore: INITIAL_STORE,
      chatError: "No API key configured",
    },
  );
}

export const test = base.extend({
  page: async ({ page }, use) => {
    await installTauriMocks(page);
    page.on("dialog", async (dialog) => {
      await dialog.dismiss();
    });
    await use(page);
  },
});

export { expect };
