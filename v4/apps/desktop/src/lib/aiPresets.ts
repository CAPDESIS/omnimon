import type { ComponentType } from "svelte";
import { BarChart2, ArrowUp, Lock, HelpCircle, Globe, AlertTriangle, Eraser, BookOpen } from "lucide-svelte";

export interface AiPreset {
  id: string;
  label: string;
  icon: ComponentType;
  prompt: string;
  category: "performance" | "security" | "network" | "general";
}

export const AI_PRESETS: AiPreset[] = [
  {
    id: "perf-general",
    label: "aiPreset.perfGeneralLabel",
    icon: BarChart2,
    prompt: "aiPreset.perfGeneralPrompt",
    category: "performance",
  },
  {
    id: "perf-top",
    label: "aiPreset.perfTopLabel",
    icon: ArrowUp,
    prompt: "aiPreset.perfTopPrompt",
    category: "performance",
  },
  {
    id: "security-audit",
    label: "aiPreset.securityAuditLabel",
    icon: Lock,
    prompt: "aiPreset.securityAuditPrompt",
    category: "security",
  },
  {
    id: "security-unknown",
    label: "aiPreset.securityUnknownLabel",
    icon: HelpCircle,
    prompt: "aiPreset.securityUnknownPrompt",
    category: "security",
  },
  {
    id: "network-traffic",
    label: "aiPreset.networkTrafficLabel",
    icon: Globe,
    prompt: "aiPreset.networkTrafficPrompt",
    category: "network",
  },
  {
    id: "network-anomaly",
    label: "aiPreset.networkAnomalyLabel",
    icon: AlertTriangle,
    prompt: "aiPreset.networkAnomalyPrompt",
    category: "network",
  },
  {
    id: "general-cleanup",
    label: "aiPreset.generalCleanupLabel",
    icon: Eraser,
    prompt: "aiPreset.generalCleanupPrompt",
    category: "general",
  },
  {
    id: "general-explain",
    label: "aiPreset.generalExplainLabel",
    icon: BookOpen,
    prompt: "aiPreset.generalExplainPrompt",
    category: "general",
  },
];

export const AI_PRESET_CATEGORY_LABELS: Record<AiPreset["category"], string> = {
  performance: "aiPreset.categoryPerformance",
  security: "aiPreset.categorySecurity",
  network: "aiPreset.categoryNetwork",
  general: "aiPreset.categoryGeneral",
};
