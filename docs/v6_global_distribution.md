# OmniMon V6 Global Distribution Architecture

## 1. Apple Notarization
*   Utilizando `actions/checkout@v4` y `tauri-apps/tauri-action@v0`.
*   Añadido proceso de firma con certificados en GitHub Secrets (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`).
*   Notarización automática con `APPLE_ID` y `APPLE_APP_SPECIFIC_PASSWORD`.

## 2. Cross-platform Native Packages
*   **Windows**: Soporte `msix` habilitado.
*   **Linux**: Paquetes `.deb`, `.rpm` y `AppImage` configurados en `tauri.conf.json`.

## 3. Homebrew Formula
*   Script Ruby proporcionado para permitir `brew install omnimon`.
