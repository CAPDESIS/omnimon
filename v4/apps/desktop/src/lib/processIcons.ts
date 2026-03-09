/**
 * Process icon mapping: assigns SVG icon paths by category/name.
 * Uses inline SVG path data to avoid external icon library dependencies.
 * Icons are 16x16 viewBox.
 */

/** Category-based icon assignments. */
const CATEGORY_ICONS: Record<string, string> = {
  // Browsers
  browser: "M8 1a7 7 0 100 14A7 7 0 008 1zm0 1.2A5.8 5.8 0 0113.8 8 5.8 5.8 0 018 13.8 5.8 5.8 0 012.2 8 5.8 5.8 0 018 2.2zm0 1.3a.75.75 0 00-.53.22L5.2 6H3.5a4.5 4.5 0 000 .5l1 2.5 2.5 1h2l2.5-1 1-2.5A4.5 4.5 0 0012.5 6h-1.7L8.53 3.72A.75.75 0 008 3.5z",
  // Terminal / Shell
  terminal: "M2 3a1 1 0 011-1h10a1 1 0 011 1v10a1 1 0 01-1 1H3a1 1 0 01-1-1V3zm2.5 1.5l3 2.5-3 2.5M8 11h4",
  // System processes
  system: "M8 1.5a1.5 1.5 0 011.415 1H12a2 2 0 012 2v8a2 2 0 01-2 2H4a2 2 0 01-2-2v-8a2 2 0 012-2h2.585A1.5 1.5 0 018 1.5zM5 7h6M5 9.5h4",
  // Database
  database: "M3 4c0-1.1 2.2-2 5-2s5 .9 5 2M3 4v2.5c0 1.1 2.2 2 5 2s5-.9 5-2V4M3 8.5V11c0 1.1 2.2 2 5 2s5-.9 5-2V8.5",
  // Network / Server
  server: "M3 2h10a1 1 0 011 1v3a1 1 0 01-1 1H3a1 1 0 01-1-1V3a1 1 0 011-1zm0 7h10a1 1 0 011 1v3a1 1 0 01-1 1H3a1 1 0 01-1-1v-3a1 1 0 011-1zm1-6.5h1M4 11.5h1",
  // Media / Creative
  media: "M6 2l8 6-8 6V2z",
  // Development
  code: "M5.5 4L2 8l3.5 4M10.5 4L14 8l-10.5 4M7.5 13l1-10",
  // Security
  security: "M8 1L2 4v4c0 4 2.6 6.5 6 8 3.4-1.5 6-4 6-8V4L8 1z",
  // File manager
  files: "M2 3.5A1.5 1.5 0 013.5 2H6l1 1.5h5.5A1.5 1.5 0 0114 5v7.5a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 012 12.5v-9z",
  // Mail
  mail: "M2 4a1 1 0 011-1h10a1 1 0 011 1v8a1 1 0 01-1 1H3a1 1 0 01-1-1V4zm1 0l5 3.5L13 4",
  // Default / Unknown
  default: "M8 2a6 6 0 100 12A6 6 0 008 2zm0 2a4 4 0 110 8 4 4 0 010-8z",
};

/** Process name to category mapping. */
const NAME_TO_CATEGORY: Array<{ pattern: RegExp; category: string }> = [
  // Browsers
  { pattern: /^(chrome|chromium|google chrome|brave|firefox|safari|edge|msedge|opera|vivaldi|arc|webkit)$/i, category: "browser" },
  // Terminals / Shells
  { pattern: /^(terminal|iterm2?|alacritty|kitty|warp|hyper|wezterm|bash|zsh|fish|sh|cmd|powershell|pwsh|WindowsTerminal|tmux|screen|ghostty)$/i, category: "terminal" },
  // Databases
  { pattern: /^(postgres|postgresql|mysql|mysqld|mongod|redis|sqlite|mariadb|couchdb|cassandra|influxd)$/i, category: "database" },
  // Servers
  { pattern: /^(nginx|httpd|apache2|node|deno|bun|caddy|traefik|envoy|gunicorn|uvicorn|puma)$/i, category: "server" },
  // Media
  { pattern: /^(spotify|vlc|mpv|quicktime|music|itunes|audacity|obs|handbrake|ffmpeg)$/i, category: "media" },
  // Code / Dev
  { pattern: /^(code|vscode|cursor|zed|neovim|nvim|vim|emacs|sublime|idea|webstorm|pycharm|xcode|android studio|fleet)$/i, category: "code" },
  // Security
  { pattern: /^(1password|bitwarden|keepass|lastpass|keychain|vault|gpg|ssh|wireguard|openvpn|littlesnitch|lulu)$/i, category: "security" },
  // File managers
  { pattern: /^(finder|explorer|nautilus|dolphin|thunar|nemo|files)$/i, category: "files" },
  // Mail
  { pattern: /^(mail|outlook|thunderbird|spark|airmail|postbox|mailmate)$/i, category: "mail" },
  // System processes (broad match, lowest priority)
  { pattern: /^(kernel_task|launchd|systemd|init|WindowServer|loginwindow|Dock|SystemUIServer|mds|mdworker|coreaudiod|bluetoothd|syslogd|cron|systemd-|dbus|udev|kworker|ksoftirqd|watchdog)$/i, category: "system" },
];

export type ProcessCategory = keyof typeof CATEGORY_ICONS;

/** Resolve category for a process by name/group. */
export function getProcessCategory(name: string, group?: string): ProcessCategory {
  // Check by group first
  if (group) {
    const g = group.toLowerCase();
    if (g === "browser") return "browser";
    if (g === "system" || g === "os") return "system";
  }

  // Match by name
  for (const rule of NAME_TO_CATEGORY) {
    if (rule.pattern.test(name)) return rule.category;
  }

  return "default";
}

/** Get the SVG path data for a process category icon. */
export function getProcessIconPath(category: ProcessCategory): string {
  return CATEGORY_ICONS[category] ?? CATEGORY_ICONS.default;
}

/** All-in-one: get icon path for a process name. */
export function iconForProcess(name: string, group?: string): string {
  return getProcessIconPath(getProcessCategory(name, group));
}

const PROCESS_ICONS: Record<string, string> = {
  "Google Chrome": "🌐",
  "chrome": "🌐",
  "Firefox": "🦊",
  "Safari": "🧭",
  "Visual Studio Code": "💻",
  "code": "💻",
  "Terminal": "⬛",
  "Finder": "📁",
  "Spotify": "🎵",
  "Slack": "💬",
  "Discord": "🎮",
  "docker": "🐳",
  "node": "💚",
  "python": "🐍",
  "rust": "🦀",
  "opencode": "🤖",
  "claude": "🟣",
};

export function getProcessIcon(name: string): string {
  const baseName = name.toLowerCase();
  for (const [key, icon] of Object.entries(PROCESS_ICONS)) {
    if (baseName.includes(key.toLowerCase())) return icon;
  }
  return "⚙️";
}

export function isNativeIconDataUrl(value?: string | null): value is string {
  return typeof value === "string" && value.startsWith("data:image/");
}

/** Get a human-readable category label. */
export function categoryLabel(category: ProcessCategory): string {
  switch (category) {
    case "browser": return "Browser";
    case "terminal": return "Terminal";
    case "system": return "System";
    case "database": return "Database";
    case "server": return "Server";
    case "media": return "Media";
    case "code": return "Development";
    case "security": return "Security";
    case "files": return "Files";
    case "mail": return "Mail";
    default: return "Application";
  }
}
