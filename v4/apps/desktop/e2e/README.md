# E2E tests (Tauri + WebdriverIO)

This suite launches the real Tauri desktop binary and drives it through `@crabnebula/tauri-driver`.

## Run

```bash
npm run test:e2e
```

Optional custom app path:

```bash
TAURI_E2E_APP_PATH="/absolute/path/to/omnimon-desktop" npm run test:e2e
```
