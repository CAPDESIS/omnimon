import { listen } from "@tauri-apps/api/event";

export interface NetworkConnection {
  process_id: number;
  process_name: string;
  protocol: string;
  local_address: string;
  local_port: number;
  remote_address: string;
  remote_port: number;
  remote_hostname: string;
  state: string;
  bytes_up: number;
  bytes_down: number;
  bytes_per_sec_up: number;
  bytes_per_sec_down: number;
}

export interface NetworkSnapshot {
  timestamp: number;
  total_bytes_per_sec_up: number;
  total_bytes_per_sec_down: number;
  active_connections: number;
  processes_with_network: number;
  connections: NetworkConnection[];
}

export interface NetworkFilter {
  protocol: string;
  process: string;
  host: string;
  hideLocalhost: boolean;
  onlyEstablished: boolean;
  minSpeed: number;
}

export function defaultFilter(): NetworkFilter {
  return {
    protocol: "",
    process: "",
    host: "",
    hideLocalhost: false,
    onlyEstablished: false,
    minSpeed: 0,
  };
}

interface NetworkState {
  snapshot: NetworkSnapshot | null;
  history: NetworkSnapshot[];
  filter: NetworkFilter;
  isCapturing: boolean;
  error: string | null;
}

let state = $state<NetworkState>({
  snapshot: null,
  history: [],
  filter: defaultFilter(),
  isCapturing: true,
  error: null,
});

function isLocalhost(ip: string): boolean {
  return ip === "127.0.0.1" || ip === "::1" || ip === "localhost";
}

function applyFilter(conn: NetworkConnection): boolean {
  if (state.filter.protocol && conn.protocol !== state.filter.protocol) return false;
  if (state.filter.process && !conn.process_name.toLowerCase().includes(state.filter.process.toLowerCase())) return false;
  if (state.filter.host && !conn.remote_hostname.toLowerCase().includes(state.filter.host.toLowerCase()) && !conn.remote_address.includes(state.filter.host)) return false;
  if (state.filter.hideLocalhost && isLocalhost(conn.remote_address)) return false;
  if (state.filter.onlyEstablished && conn.state !== "ESTABLISHED") return false;
  if (state.filter.minSpeed > 0 && Math.max(conn.bytes_per_sec_up, conn.bytes_per_sec_down) < state.filter.minSpeed * 1024) return false;
  return true;
}

export function getNetworkState() {
  return state;
}

export function getFilteredConnections(): NetworkConnection[] {
  return state.snapshot?.connections.filter(applyFilter) ?? [];
}

export function getPerProcessSummary(): Array<{
  name: string;
  connectionsCount: number;
  totalUp: number;
  totalDown: number;
  topDest: string;
}> {
  const connections = getFilteredConnections();
  const map = new Map<string, {
    name: string;
    connectionsCount: number;
    totalUp: number;
    totalDown: number;
    topDest: string;
  }>();

  for (const conn of connections) {
    const pName = conn.process_name || "Unknown";
    let entry = map.get(pName);
    if (!entry) {
      entry = { name: pName, connectionsCount: 0, totalUp: 0, totalDown: 0, topDest: conn.remote_hostname || conn.remote_address };
      map.set(pName, entry);
    }
    entry.connectionsCount++;
    entry.totalUp += conn.bytes_per_sec_up;
    entry.totalDown += conn.bytes_per_sec_down;
  }

  return Array.from(map.values()).sort((a, b) => (b.totalUp + b.totalDown) - (a.totalUp + a.totalDown));
}

export function getTotalUp(): number {
  return state.snapshot?.total_bytes_per_sec_up ?? 0;
}

export function getTotalDown(): number {
  return state.snapshot?.total_bytes_per_sec_down ?? 0;
}

export async function initNetworkListener() {
  const unlisten = await listen<NetworkSnapshot>("network-update", (event: { payload: NetworkSnapshot }) => {
    state.snapshot = event.payload;
    state.history = [...state.history.slice(-59), event.payload];
  });
  return unlisten;
}
