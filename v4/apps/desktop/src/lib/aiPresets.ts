export interface AiPreset {
  id: string;
  label: string;
  icon: string;
  prompt: string;
  category: "performance" | "security" | "network" | "general";
}

export const AI_PRESETS: AiPreset[] = [
  {
    id: "perf-general",
    label: "Rendimiento general",
    icon: "📊",
    prompt: "Analiza el rendimiento general del sistema. ¿Hay procesos consumiendo recursos excesivos? ¿Hay algo anormal?",
    category: "performance",
  },
  {
    id: "perf-top",
    label: "¿Qué consume más?",
    icon: "🔝",
    prompt: "¿Cuáles son los 5 procesos que más CPU y RAM están consumiendo? ¿Alguno es innecesario?",
    category: "performance",
  },
  {
    id: "security-audit",
    label: "Auditoría de seguridad",
    icon: "🔒",
    prompt: "Realiza una auditoría rápida de seguridad. ¿Hay procesos sospechosos, conexiones a IPs inusuales, o puertos abiertos que no deberían estar?",
    category: "security",
  },
  {
    id: "security-unknown",
    label: "Procesos desconocidos",
    icon: "❓",
    prompt: "Identifica procesos que no reconozcas o que parezcan sospechosos. Explica qué hace cada uno.",
    category: "security",
  },
  {
    id: "network-traffic",
    label: "Analizar tráfico de red",
    icon: "🌐",
    prompt: "Analiza el tráfico de red actual. Dame las top 10 conexiones por ancho de banda, puertos abiertos relevantes, y procesos con más tráfico. Explica en lenguaje simple qué está pasando en mi red.",
    category: "network",
  },
  {
    id: "network-anomaly",
    label: "Detección de anomalías",
    icon: "🚨",
    prompt: "Revisa estas conexiones y dime si hay algo sospechoso. Presta especial atención a conexiones a IPs desconocidas, puertos inusuales, procesos con mucho tráfico inesperado. Dame una lista estructurada de hallazgos con severidad y si aplica, recomiéndame cerrar la conexión.",
    category: "network",
  },
  {
    id: "general-cleanup",
    label: "Sugerencias de limpieza",
    icon: "🧹",
    prompt: "Sugiere procesos que podría cerrar para liberar recursos. No incluyas procesos del sistema operativo ni esenciales.",
    category: "general",
  },
  {
    id: "general-explain",
    label: "Explicar sistema",
    icon: "📖",
    prompt: "Dame un resumen del estado actual del sistema: CPU, RAM, disco, red. ¿Está todo normal?",
    category: "general",
  },
];

export const AI_PRESET_CATEGORY_LABELS: Record<AiPreset["category"], string> = {
  performance: "Rendimiento",
  security: "Seguridad",
  network: "Red",
  general: "General",
};
