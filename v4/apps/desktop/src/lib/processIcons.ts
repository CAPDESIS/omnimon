import {
  Globe, Terminal, Folder, Music, MessageSquare,
  Gamepad2, Container, Hexagon, Code, Cpu,
  Shield, Cog, Monitor, Database, Wifi
} from "lucide-svelte";
import type { ComponentType } from "svelte";

const PROCESS_ICONS: Record<string, ComponentType> = {
  "chrome": Globe,
  "google chrome": Globe,
  "firefox": Globe,
  "safari": Globe,
  "code": Code,
  "visual studio code": Code,
  "terminal": Terminal,
  "iterm": Terminal,
  "finder": Folder,
  "spotify": Music,
  "slack": MessageSquare,
  "discord": Gamepad2,
  "docker": Container,
  "node": Hexagon,
  "python": Code,
  "rust": Cog,
  "opencode": Monitor,
  "claude": Cpu,
};

export function getProcessIconComponent(name: string): ComponentType {
  if (!name) return Cog;
  const lower = name.toLowerCase();
  for (const [key, icon] of Object.entries(PROCESS_ICONS)) {
    if (lower.includes(key)) return icon;
  }
  return Cog; // Default
}
