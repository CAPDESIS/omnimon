export interface ProcessEntry {
  pid: number;
  name: string;
  exec_name: string;
  ram_mb: number;
  cpu_pct: number;
  uptime: string;
  group: string;
  is_system: boolean;
  idle: boolean;
  state: string;
}

export interface SystemStats {
  ram_total_gb: number;
  ram_used_pct: number;
  swap_used_mb: number;
  total_processes: number;
}

export interface Metrics {
  processes: ProcessEntry[];
  stats: SystemStats;
}

export interface BrowserTab {
  id: string;
  title: string;
  url: string;
  browser: "Chrome" | "Safari";
}
