# OmniMon UI Audit — Transparencia + Z-Index + Layout

> **Reglas de transparencia:** NO rgba() en backgrounds (solo modal overlay y box-shadow), NO backdrop-filter: blur(),
> NO opacity<1 en contenido, NO color-mix con transparent, fondos siempre solidos.
> color-mix con porcentajes >= 25% para fondos coloreados.

> **Reglas de z-index:** Todos los modales a z-index: 1000. Toasts/SmartAlerts a 9999. Dropdowns/popovers < 200.
> Solo UN modal abierto a la vez (closeAllModals() antes de abrir uno nuevo).

## Estado: COMPLETADO - 44/44 archivos revisados

### Build: OK | Tests: 45/45 suites, 674/674 tests

---

## Auditoría de transparencia

| # | Archivo | Estado | Notas |
|---|---------|--------|-------|
| 1 | App.svelte | PASS | rgba solo en modal backdrop (permitido) |
| 2 | lib/theme.ts | PASS | rgba solo en box-shadow (permitido) |
| 3 | ProcessTable.svelte | CORREGIDO | Badges 14-15%→28%, rank-pulse con color-mix 30% |
| 4 | AIChat.svelte | CORREGIDO | Tool badges 15%→28%, preview 8%→25%, chips 10-14%→25-28% |
| 5 | NetworkMap.svelte | CORREGIDO | globalAlpha eliminado→color solido, CSS yellow 8%→25% |
| 6 | SystemDashboard.svelte | CORREGIDO | Canvas rgba(0.1) eliminado→color solido oscurecido |
| 7 | Plugins.svelte | CORREGIDO | Banners 16-18%→28%, metric tags 12%→25% |
| 8 | SecurityReportView.svelte | CORREGIDO | Accordion 8%→25%, badges 12%→28% |
| 9 | ChromeTabManager.svelte | CORREGIDO | Kill/close btns 10-15%→28% |
| 10 | SecurityBadge.svelte | CORREGIDO | Hex alpha `1a` en inline style→color-mix 28%, danger 15%→28% |
| 11 | ProcessDetailsModal.svelte | CORREGIDO | Tab bg 3%→25% |
| 12 | AlertPanel.svelte | CORREGIDO | Tags 14%→28% |
| 13 | NetworkAlertConfig.svelte | CORREGIDO | Tags 14%→28% |
| 14 | InfoPopover.svelte | CORREGIDO | Background 8%→25% |
| 15 | ProfileSettings.svelte | CORREGIDO | Active profile 10%→25% |
| 16 | AppToolbar.svelte | PASS | rgba solo en box-shadow, color-mix 14% solo en box-shadow |
| 17 | AiCommandBar.svelte | PASS | Sin violaciones |
| 18 | AiInsightCard.svelte | PASS | Sin violaciones |
| 19 | ContextAiChat.svelte | PASS | opacity 0/1 solo en @keyframes (permitido) |
| 20 | Button.svelte | CORREGIDO | .is-active bg 18%→25% |
| 21 | IconButton.svelte | CORREGIDO | Danger bg 18%→25% |
| 22 | ModalShell.svelte | PASS | rgba(0,0,0,0.7) backdrop (permitido), rgba en box-shadow |
| 23 | HelpCenterModal.svelte | PASS | rgba(0,0,0,0.7) backdrop, color-mix 22% en border (permitido) |
| 24 | SystemMetricModal.svelte | PASS | background:transparent en sort-button ghost (permitido) |
| 25 | ConfirmDialog.svelte | PASS | Sin usos de rgba/opacity/backdrop-filter |
| 26 | StatusBar.svelte | PASS | Sin violaciones |
| 27 | SmartAlerts.svelte | PASS | rgba solo en box-shadow |
| 28 | ToastContainer.svelte | PASS | rgba solo en box-shadow |
| 29 | Skeleton.svelte | PASS | Sin violaciones |
| 30 | EmptyState.svelte | PASS | Sin violaciones |
| 31 | ThemeSelector.svelte | PASS | transparent en botones ghost, rgba en box-shadow |
| 32 | CloudSync.svelte | PASS | Sin violaciones |
| 33 | Automations.svelte | CORREGIDO | Convertido de div inline a modal propio con backdrop |
| 34 | ProfilePanel.svelte | CORREGIDO | .profile-card.selected bg 14%→25% |
| 35 | ConnectionsTable.svelte | PASS | Sin violaciones |
| 36 | ProcessNetworkView.svelte | PASS | Sin violaciones |
| 37 | NetworkDashboard.svelte | PASS | rgba solo en box-shadow |
| 38 | layout/AppHeader.svelte | PASS | Sin violaciones |
| 39 | layout/AppStatusBar.svelte | PASS | Sin violaciones |
| 40 | layout/AppSidebar.svelte | PASS | Sin violaciones (deprecated, no se usa) |
| 41 | layout/NavigationTabs.svelte | PASS | transparent en boton ghost (permitido) |
| 42 | layout/AppLayout.svelte | PASS | Sin violaciones |
| 43 | layout/AIConfigPanel.svelte | PASS | Sin violaciones |
| 44 | network/ConnectionDetail.svelte | PASS | rgba solo en box-shadow |

---

## Auditoría de z-index (estandarizada)

| Componente | z-index | Tipo | Estado |
|------------|---------|------|--------|
| ModalShell.svelte | 1000 | Modal backdrop | CORREGIDO (era 230) |
| HelpCenterModal.svelte | 1000 | Modal backdrop | CORREGIDO (era 220) |
| Plugins.svelte | 1000 | Modal backdrop | CORREGIDO (era 120) |
| Automations.svelte | 1000 | Modal backdrop | NUEVO (antes era inline, sin backdrop) |
| NetworkAlertConfig.svelte | 1000 | Modal backdrop | CORREGIDO (era 1200) |
| App.svelte .backdrop | 1000 | Modal backdrop (Settings) | OK |
| SmartAlerts.svelte | 9999 | Notificaciones flotantes | OK |
| ToastContainer.svelte | 9999 | Notificaciones flotantes | OK |
| AlertPanel.svelte | 200 | Panel flotante (no modal) | OK |
| ThemeSelector.svelte | 100 | Dropdown | OK |
| NetworkMap.svelte | 100 | .connection-detail-overlay | OK |
| InfoPopover.svelte | 40 | Popover local | OK |
| AIChat.svelte | 10 | .scroll-to-bottom button | OK |
| ProcessTable.svelte | 2 | Sticky header | OK |
| NetworkMap.svelte | 1 | Sticky header | OK |
| ChromeTabManager.svelte | 1 | Sticky header | OK |
| SystemMetricModal.svelte | 1 | Sticky header | OK |

---

## Escala de z-index del proyecto

```
z-index: 1      → Sticky headers (dentro de scrollable containers)
z-index: 10     → Floating buttons dentro de componentes
z-index: 40     → Popovers locales (InfoPopover)
z-index: 100    → Dropdowns, overlays dentro de tabs (ThemeSelector, ConnectionDetail)
z-index: 200    → Panels flotantes persistentes (AlertPanel)
z-index: 1000   → TODOS los modales (backdrop + dialog)
z-index: 9999   → Toasts y SmartAlerts (siempre visibles sobre modales)
```

---

## Exclusión mutua de modales

`closeAllModals()` en App.svelte cierra todos los modales antes de abrir uno nuevo:
- `detailProcess` (ProcessDetailsModal)
- `showSettings` (Settings)
- `showSecurityReport` (SecurityReport)
- `showAutomations` (Automations)
- `showPlugins` (Plugins)
- `showHelpCenter` (HelpCenter)
- `activeMetricModal` (SystemMetricModal)

---

## Resumen de cambios

- **Total archivos:** 44
- **PASS (sin cambios):** 25
- **CORREGIDOS (transparencia):** 18
- **CORREGIDOS (z-index):** 5
- **CORREGIDOS (layout/modal):** 2 (Automations, App.svelte tab-pane position)
- **Violaciones restantes:** 0
