# E2E tests (Tauri + WebdriverIO)

This suite launches the real Tauri desktop binary and drives it through `@crabnebula/tauri-driver`.

## Prerequisites

- macOS: `tauri-plugin-automation` enabled in Tauri app and `CN_API_KEY` exported
- Linux: `webkit2gtk-driver` in PATH
- Windows: `msedgedriver.exe` in PATH

## Run

```bash
npm run test:e2e
```

On macOS without `CN_API_KEY`, the runner skips by default. To enforce failure when prerequisites are missing:

```bash
E2E_STRICT=1 npm run test:e2e
```

macOS example:

```bash
export CN_API_KEY="<your-crabnebula-key>"
npm run test:e2e
```

Optional custom app path:

```bash
TAURI_E2E_APP_PATH="/absolute/path/to/omnimon-desktop" npm run test:e2e
```
