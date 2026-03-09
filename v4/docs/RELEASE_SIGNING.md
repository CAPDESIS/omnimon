# Proceso de Firma de Releases — OmniMon

## Resumen

OmniMon firma digitalmente todos los artefactos de release usando Ed25519
(NIST FIPS 186-5, RFC 8032). La verificación es doble:

1. **SHA-256** — Integridad del archivo (NIST FIPS 180-4)
2. **Ed25519** — Autenticidad del firmante (solo quien posee la clave privada)

Esto cumple con **NIST SP 800-53 SI-7** (Software, Firmware, and Information
Integrity).

## Flujo de Firma

```
┌─────────────┐     ┌──────────────┐     ┌────────────────┐
│  cargo build │ ──> │ sign-release  │ ──> │  GitHub Release │
│  --release   │     │  (Ed25519)   │     │  + .sig.json    │
└─────────────┘     └──────────────┘     │  + SHA256SUMS   │
                                          │  + releases.json│
                                          └────────────────┘
```

## Comandos CLI

### Generar par de claves Ed25519

```bash
omnimon release generate-keypair
```

- La **clave privada** se almacena en el keyring nativo del OS
  (macOS Keychain / Windows Credential Manager / Linux Secret Service)
- La **clave pública** (base64) se imprime en stdout

### Firmar un artefacto

```bash
# Usando clave del keyring
omnimon release sign ./target/release/omnimon --version 6.0.1

# Usando archivo de clave
omnimon release sign ./target/release/omnimon --version 6.0.1 --key-file signing.key
```

Genera `<archivo>.sig.json` con la estructura:

```json
{
  "version": "6.0.1",
  "sha256": "abc123...",
  "signature_b64": "def456...",
  "public_key_b64": "ghi789..."
}
```

### Verificar un artefacto

```bash
# Usando la pubkey embebida en el .sig.json
omnimon release verify ./omnimon --sig ./omnimon.sig.json

# Usando una pubkey específica (más seguro)
omnimon release verify ./omnimon --sig ./omnimon.sig.json --pubkey "base64pubkey..."
```

### Calcular checksum SHA-256

```bash
omnimon release checksum ./target/release/omnimon
# Output: a1b2c3d4...  ./target/release/omnimon
```

### Generar manifiesto de release

```bash
omnimon release manifest --version 6.0.1 --dir ./target/release/
```

Genera `releases.json` con todos los artefactos del directorio, firmados
individualmente y con una firma global del manifiesto.

### Verificar manifiesto

```bash
omnimon release verify-manifest releases.json --pubkey "base64pubkey..."
```

## Almacenamiento de Claves

| Componente | Ubicación | Acceso |
|------------|-----------|--------|
| Clave privada (desarrollo) | OS Keyring (`omnimon_release/ed25519_signing_key`) | Solo el usuario local |
| Clave privada (CI) | GitHub Secret `ED25519_SIGNING_KEY` (base64) | Solo CI runners |
| Clave pública | `tauri.conf.json` → `plugins.updater.pubkey` | Embebida en el binario |

## Integración CI/CD

El pipeline de GitHub Actions firma automáticamente cuando:

1. Se hace push de un tag `v*` o se ejecuta `workflow_dispatch`
2. El job `release` construye los binarios con `tauri-action`
3. El job `sign-release` descarga los artefactos y:
   - Calcula SHA-256 de cada archivo → `SHA256SUMS.txt`
   - Firma cada artefacto con Ed25519 → `*.sig.json`
   - Genera `releases.json` con metadatos de todos los artefactos
   - Sube todo como assets del GitHub Release

### Configurar secretos en GitHub

```bash
# 1. Generar keypair localmente
omnimon release generate-keypair
# Copiar la clave pública (base64) impresa en stdout

# 2. Exportar la clave privada para CI
# La clave está en el keyring. Para extraerla y codificarla en base64:
# (varía según el OS, consultar SECURITY_KEYS.md)

# 3. En GitHub → Settings → Secrets → Actions:
# - ED25519_SIGNING_KEY = base64 de la clave privada
```

## Verificación por Usuarios Finales

Los usuarios pueden verificar la integridad de un download:

```bash
# 1. Descargar el binario y su archivo .sig.json desde GitHub Releases
# 2. Verificar con la clave pública del proyecto:
omnimon release verify OmniMon-6.0.1-macOS-Universal.dmg \
  --sig OmniMon-6.0.1-macOS-Universal.dmg.sig.json \
  --pubkey "CLAVE_PUBLICA_BASE64"
```

O manualmente con los checksums:

```bash
# Verificar integridad con SHA-256
sha256sum -c SHA256SUMS.txt
```

## Controles NIST Implementados

| Control | Descripción | Implementación |
|---------|-------------|----------------|
| SI-7 | Software/Firmware Integrity | Ed25519 signing + SHA-256 checksums |
| SI-7(1) | Integrity Checks | `verify_release()`, `verify_binary_integrity()` |
| SI-7(6) | Cryptographic Protection | Ed25519 (FIPS 186-5) + SHA-256 (FIPS 180-4) |
| SC-12 | Key Establishment | OS Keyring + GitHub Secrets |
| SC-13 | Cryptographic Protection | `ed25519-dalek` (audited Rust crate) |

## Archivos Relevantes

| Archivo | Descripción |
|---------|-------------|
| `crates/core/src/crypto.rs` | Funciones Ed25519, SHA-256, manifests |
| `crates/cli/src/main.rs` | Subcomando `release` |
| `scripts/sign-release.sh` | Script de firma para CI (usa openssl) |
| `.github/workflows/omnimon-ci.yml` | Pipeline con firma automatizada |
| `releases.json` | Manifiesto de releases (esquema de ejemplo) |
| `docs/SECURITY_KEYS.md` | Gestión de claves de updater Tauri |
