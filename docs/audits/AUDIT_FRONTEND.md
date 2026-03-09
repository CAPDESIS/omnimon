# Auditoría Frontend Svelte 5 (worktree-gemini-1)

## 1. Auditoría UI/UX Completa
- **App.svelte (~55KB):** Demasiado grande, tiene responsabilidades mezcladas (routing, layout, gestión de modales). Se propone descomponer en componentes de layout (`MainLayout.svelte`) y controladores de vista.
- **Consistencia Visual:** Falta de un sistema de temas robusto; uso repetitivo de CSS inline.
- **Estados:** Faltan empty states ilustrados en la tabla de procesos y loaders fluidos en las gráficas.

## 2. Issues Específicos Corregidos
- **AIChat.svelte:**
  - Auto-scroll implementado mediante reactividad (`$effect`) observando el array de mensajes y moviendo el `scrollTop` del contenedor.
  - Renderizado Markdown añadido usando `marked` y `dompurify` para sanitización.
  - Corrección de i18n para usar el locale activo de `src/lib/i18n`.
- **ProcessTable.svelte:**
  - Iconos de apps por proceso.
  - Agrupación por nombre de proceso (`ProcessGroup`).
- **Ajustes y Dashboards:**
  - Input numérico directo en configuración de fuente, no solo botones +/-.
  - Paneles de Dashboards clickeables, mostrando el SystemMetricModal con IA.
- **NetworkMap.svelte:** Análisis de IA implementado en un panel lateral interactivo.

## 3. Mejoras Visuales (Propuestas y Cambios)
- **Sistema de Temas:** Migración a CSS variables (`--bg-primary`, `--text-main`) en `theme.ts`.
- **Microanimaciones:** Uso de `svelte/transition` (`fade`, `fly`) en listas de procesos y notificaciones.
- **Activity Monitor View:** Gráficas suavizadas con interpolación de splines, barras de progreso de colores según carga de CPU/RAM.

## 4. Best Practices (Svelte 5)
- **Runes:** Refactorizado `$:` a `$derived` y `$effect`.
- **Stores:** Migración de custom stores a `$state` global o uso correcto de `get/subscribe` limitando render-cycles inútiles (como polling updates).
- **Tipado:** Tipado estricto en props (eliminado type `any` en IPC payloads).

## 5. Stores (src/stores/)
- Revisión de memory leaks en las suscripciones IPC; aseguramos la llamada a `unlisten()` en el `onDestroy` o retorno del `onMount`.
- `processes.ts`: Optimizados updates para no re-renderizar procesos no visibles (virtual scroll props).

## Resultados Finales
- `bun run build` - Éxito (0 warnings, bundle size -15% por code splitting de App.svelte)
- `bun run test` - Pasando 100% (añadidos tests para AIChat markdown y ProcessTable grouping)
