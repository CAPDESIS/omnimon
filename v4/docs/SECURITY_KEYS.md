# Generación de claves de firma para auto-update (Tauri Updater)

OmniMon usa CrabNebula CDN para distribuir actualizaciones firmadas con Ed25519.

## Generar par de claves

```bash
# Genera clave privada (proteger con contraseña) y muestra la clave pública
bunx @tauri-apps/cli signer generate -w ~/.tauri/omnimon.key
```

## Configurar la clave pública

Copia la clave pública generada (una línea base64) al campo `pubkey` en:

```
apps/desktop/src-tauri/tauri.conf.json → plugins.updater.pubkey
```

## Firmar los binarios de release

En CI/CD, configura las variables de entorno:

```
TAURI_SIGNING_PRIVATE_KEY=contenido-de-~/.tauri/omnimon.key
TAURI_SIGNING_PRIVATE_KEY_PASSWORD=tu-contraseña
```

## Seguridad

- **NUNCA** commitear la clave privada al repositorio.
- La clave privada debe estar en un secret manager (GitHub Secrets, etc.).
- La clave pública SÍ se commitea en `tauri.conf.json` — es necesaria para
  que el cliente verifique la autenticidad de las actualizaciones.
- Sin una pubkey válida, el updater no puede verificar firmas y un atacante
  con capacidad MITM podría servir binarios maliciosos.
