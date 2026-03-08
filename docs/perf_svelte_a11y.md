# Perf Svelte + Strict A11y

## Objetivo

Reducir trabajo de render inicial en Svelte 5, evitar recomputaciones/polling innecesario cuando una vista pesada no esta visible y llevar la UI a una pasada estricta de accesibilidad sin warnings de `svelte-check`.

## Arquitectura aplicada

### 1. Lazy loading de componentes pesados

- `v4/apps/desktop/src/App.svelte`
  - Se movieron a carga diferida por `import()` los componentes mas costosos o poco frecuentes:
    - `ChromeTabManager`
    - `AIChat`
    - `NetworkMap`
    - `ProcessDetailsModal`
    - `SecurityReportView`
    - `HelpCenterModal`
    - `SystemMetricModal`
    - `Automations`
    - `CloudSync`
  - La carga inicial usa `#await` y placeholders ligeros con `SkeletonBlock`.
  - `IntersectionObserver` dispara la carga de paneles visibles en viewport para no bloquear el primer paint.

### 2. Polling reactivo por demanda

- `v4/apps/desktop/src/stores/processes.ts`
  - Se separo el polling base de metricas del polling opcional de:
    - tabs del navegador
    - telemetria de red
  - Nuevo control `setPollingTarget()` habilita esos ciclos solo cuando la UI que los consume se carga.
  - Resultado: si el usuario no entra al mapa de red, no se refresca la telemetria de red continuamente.

### 3. Reduccion de trabajo invisible en Network Map

- `v4/apps/desktop/src/components/NetworkMap.svelte`
  - `processNodes`, `summaryCards` y `sortedConnections` ya no computan igual cuando el panel esta colapsado o la pestana activa no necesita esos datos.
  - Se elimino el skeleton transitorio que forzaba churn visual al cambiar de pestana.
  - La grafica `lightweight-charts` sigue cargando lazy solo al abrir la pestana `traffic`.

### 4. A11y estricta en modales y tablas

- Se normalizo el patron de dialogo en:
  - `v4/apps/desktop/src/App.svelte`
  - `v4/apps/desktop/src/components/SystemMetricModal.svelte`
  - `v4/apps/desktop/src/components/ProcessDetailsModal.svelte`
  - `v4/apps/desktop/src/components/HelpCenterModal.svelte`
  - `v4/apps/desktop/src/components/SecurityReportView.svelte`
- Ajustes principales:
  - cierre por backdrop sin `svelte-ignore`
  - `role="dialog"`, `aria-modal`, `aria-labelledby`
  - focus trap consistente
  - restauracion de foco al cerrar ajustes

- `v4/apps/desktop/src/components/ProcessTable.svelte`
  - los `th` con sort ahora usan `button` reales
  - el encabezado de grupo ahora usa un `button` interno accesible
  - filas de procesos aceptan teclado para seleccionar/abrir detalle

- `v4/apps/desktop/src/components/SystemMetricModal.svelte`
  - sort accesible en la tabla interna
  - sparkline SVG con `aria-label` resumido
  - `NetworkMap` dentro del modal ahora tambien se carga lazy

- `v4/apps/desktop/src/components/NetworkMap.svelte`
  - tabs con `aria-controls`, `aria-selected` y `tabindex` coherentes
  - canvas del mapa con descripcion textual accesible
  - headers de tabla convertidos a botones accesibles

### 5. Contraste WCAG AA en dark/cyberpunk

- `v4/apps/desktop/src/lib/theme.ts`
  - se elevaron los tonos de `--fg-dim` en temas oscuros para mejorar legibilidad y contraste sobre fondos oscuros.

## Stores revisados

### `alerts.ts`

- Se corrigio tipado y payload de contexto para evitar errores de chequeo.
- El promedio CPU usado en alertas inteligentes ahora se deriva de procesos visibles disponibles en memoria en vez de depender de un campo inexistente.

### `preferences.ts`

- Se audito que no introdujera repaints innecesarios; no requirio cambio estructural para esta mision.

## Archivos tocados

- `v4/apps/desktop/src/App.svelte`
- `v4/apps/desktop/src/components/HelpCenterModal.svelte`
- `v4/apps/desktop/src/components/NetworkMap.svelte`
- `v4/apps/desktop/src/components/ProcessDetailsModal.svelte`
- `v4/apps/desktop/src/components/ProcessTable.svelte`
- `v4/apps/desktop/src/components/SecurityReportView.svelte`
- `v4/apps/desktop/src/components/SystemMetricModal.svelte`
- `v4/apps/desktop/src/components/__tests__/NetworkMap.test.ts`
- `v4/apps/desktop/src/lib/focusTrap.ts`
- `v4/apps/desktop/src/lib/theme.ts`
- `v4/apps/desktop/src/stores/alerts.ts`
- `v4/apps/desktop/src/stores/processes.ts`

## Validacion esperada

- `bunx svelte-check`
- `bun run build`
- `bun run test`

## Resultado esperado

- Menor costo de bundle y trabajo inicial.
- Menos polling innecesario cuando paneles pesados no estan montados.
- Cero warnings de accesibilidad en `svelte-check`.
- Navegacion por teclado mas consistente en modales, tablas y tabs.
