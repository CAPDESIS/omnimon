# OS Integration Plan (OmniMon v4)

## Objective

Implement native macOS desktop integration for OmniMon v4 with three capabilities:

1. Background daemon-style startup without showing the main window.
2. Native tray/menu-bar icon controls for window visibility and app lifecycle.
3. User-controlled auto-start on login via official Tauri v2 autostart plugin.

## Design

### 1) Background startup

- Configure main window in `tauri.conf.json` as hidden by default (`visible: false`).
- Keep watcher and alert threads initialized in Tauri `setup` so metrics continue while the UI window is hidden.
- Emit `window-visibility` events from Rust whenever tray/window actions show/hide the main window so Svelte polling behavior remains consistent.

### 2) Tray icon behavior

- Keep tray menu actions:
  - `Dashboard` -> show and focus main window
  - `Configuración` -> show/focus + emit `open-settings`
  - `Salir` -> exit app
- Add left-click toggle behavior on tray icon:
  - if window visible -> hide
  - if window hidden -> show + focus
- Keep close interception (`CloseRequested`) to hide instead of quitting.

### 3) Auto-start integration

- Add `tauri-plugin-autostart` in Rust and initialize in builder.
- Add Tauri IPC commands:
  - `get_autostart_enabled() -> bool`
  - `set_autostart_enabled(enabled: bool) -> ()`
- Use official plugin manager APIs; no custom launch-agent scripts.

### 4) Frontend preference UX

- Add an `Autostart at login` toggle in Svelte Settings modal (`App.svelte`).
- On app mount, read current autostart state from IPC and reflect it in UI.
- On toggle change, call Rust IPC to persist plugin state.
- Handle failures by showing a non-blocking error message in settings area.

## Validation plan

- `cargo check` in `v4/apps/desktop/src-tauri` to validate Rust compile/lints baseline.
- `npm run build` for frontend type+bundle sanity after IPC additions.
- Manual smoke behavior:
  - app starts hidden,
  - tray left-click toggles visibility,
  - tray menu actions work,
  - autostart toggle persists and reflects actual plugin state.
