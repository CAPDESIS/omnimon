# Auditoría de Seguridad — OmniMon v6.0.1

> Estado actual 2026-06-29: este reporte es una captura histórica del
> 2026-03-08. La app actual está en v6.7.0, el updater `pubkey` ya está
> poblado, el audit frontend de CI dejó de ser `continue-on-error`, y `bun
> audit` en `v4/apps/desktop` reportó `No vulnerabilities found`. Las tablas
> históricas que mencionan pubkey vacío o audit frontend no bloqueante no
> representan el estado actual.

**Fecha:** 2026-03-08
**Auditor:** Agente OPUS-SECURITY (Claude Opus)
**Branch:** `audit/opus-security`
**Alcance:** Criptografía, MITRE ATT&CK, NIST SP 800-53, IPC, dependencias, CI/CD, AI/LLM

---

## Resumen Ejecutivo

Se realizó una auditoría exhaustiva de seguridad sobre el codebase de OmniMon v6.0.1, cubriendo 15+ archivos fuente en Rust y TypeScript, el pipeline CI/CD, y las 676 dependencias Rust + 664 paquetes npm.

### Métricas Globales

| Categoría | Cantidad |
|-----------|----------|
| Vulnerabilidades **CRÍTICAS** | 0 |
| Vulnerabilidades **ALTAS** | 2 |
| Vulnerabilidades **MEDIAS** | 14 |
| Vulnerabilidades **BAJAS** | 12 |
| Hallazgos **INFORMATIVOS** | 4 |
| Controles positivos identificados | 15 |

**Postura general: BUENA (7/10)** — Arquitectura de seguridad sólida con defensa en profundidad. Las vulnerabilidades encontradas son remediables sin rediseño arquitectónico.

---

## Tabla de Vulnerabilidades

| ID | Severidad | CWE | Módulo | Descripción | Línea |
|----|-----------|-----|--------|-------------|-------|
| SEC-01 | **ALTA** | CWE-1059 | security.rs | Técnica MITRE T1043 obsoleta/revocada desde ATT&CK v8 | 88 |
| SEC-02 | **ALTA** | CWE-770 | lib.rs | `apply_ai_rules` sin rate limiting — permite inyección masiva de reglas | 395 |
| SEC-03 | MEDIA | CWE-316 | crypto.rs | Claves criptográficas no se zeroizan de memoria tras uso | - |
| SEC-04 | MEDIA | CWE-327 | crypto.rs | Ausencia de Key Derivation Function (PBKDF2/HKDF/Argon2) | - |
| SEC-05 | MEDIA | CWE-862 | rules_engine.rs | `upsert_rules_from_ai_json` reemplaza TODAS las reglas (no merge) | 216 |
| SEC-06 | MEDIA | CWE-77/74 | ai.rs | Detección de prompt injection por blocklist trivialmente evadible | 27-46 |
| SEC-07 | MEDIA | CWE-74 | ai.rs | `analyze_with_ai()` no aplica `check_prompt_injection()` | 366 |
| SEC-08 | MEDIA | CWE-20 | ai.rs | Tool calling no valida estructura de argumentos rigurosamente | 649 |
| SEC-09 | MEDIA | CWE-312 | lib.rs (Tauri) | Fallback de API keys a Tauri Store sin cifrar | 377-381 |
| SEC-10 | MEDIA | CWE-347 | tauri.conf.json | Updater `pubkey` vacío — updates sin verificación de firma | 40 |
| SEC-11 | MEDIA | CWE-770 | lib.rs | `kill_processes` consume 1 token de rate limit para N PIDs ilimitados | 316-330 |
| SEC-12 | MEDIA | CWE-345 | audit.rs | Base de datos CVE local sin verificación de integridad/firma | 78 |
| SEC-13 | MEDIA | CWE-829 | CI/CD | GitHub Actions no pinneadas por SHA (`trufflehog@main` especialmente) | omnimon-ci.yml |
| SEC-14 | MEDIA | CWE-347 | CI/CD | Artefactos de release con SHA-256 pero sin firma Ed25519 | omnimon-ci.yml |
| SEC-15 | MEDIA | CWE-1059 | security.rs | Cobertura MITRE incompleta — faltan T1059, T1071, T1053 | - |
| SEC-16 | MEDIA | CWE-862 | lib.rs | Comandos de automations/plugins sin rate limiting | automations.rs |
| SEC-17 | BAJA | CWE-208 | crypto.rs | Comparación SHA-256 sin tiempo constante | 136 |
| SEC-18 | BAJA | CWE-693 | killer.rs | Cobertura incompleta de procesos protegidos del OS | 9-68 |
| SEC-19 | BAJA | CWE-400 | ai.rs | Cache de respuestas AI sin límite de tamaño ni TTL | 15 |
| SEC-20 | BAJA | CWE-78 | network.rs | `lsof`/`netstat` invocados sin ruta absoluta | 476 |
| SEC-21 | BAJA | CWE-427 | network.rs | Ruta eBPF configurable via env var sin verificación de integridad | - |
| SEC-22 | BAJA | CWE-20 | lib.rs | `apply_ai_rules` sin límite de tamaño ni número de reglas | 395-397 |
| SEC-23 | BAJA | CWE-20 | ipc.ts | `tool_call` en `ipcAiChat` no se valida en profundidad | 388-389 |
| SEC-24 | BAJA | CWE-319 | browser.rs | CDP usa HTTP sin cifrar (inherente al protocolo, solo localhost) | - |
| SEC-25 | BAJA | CWE-918 | browser.rs | Funciones públicas CDP aceptan `base_url` arbitrario | - |
| SEC-26 | BAJA | CWE-20 | lib.rs | `save_cloud_key` IPC sin validación de formato | 530 |
| SEC-27 | BAJA | CWE-16 | tauri.conf.json | Falta directiva `img-src` explícita en CSP | 31 |
| SEC-28 | BAJA | CWE-362 | watcher.rs | Estado global sin autenticación de escritor | 17 |
| INFO-01 | INFO | - | crypto.rs | Campo `public_key_b64` en `ReleaseSignature` potencialmente confuso | 80-85 |
| INFO-02 | INFO | - | security.rs | Confidence score estático (0.7/0.9) sin contexto multi-señal | 146-149 |
| INFO-03 | INFO | - | CI/CD | Job `security` con `continue-on-error: true` | omnimon-ci.yml |
| INFO-04 | INFO | - | tauri.conf.json | Ollama local funciona correctamente via proxy del backend Rust | - |

---

## 1. Dependencias y CVEs

### 1.1 Vulnerabilidades Rust (cargo audit)

**Herramienta:** `cargo-audit 0.22.1` (946 advisories cargados)

#### Vulnerabilidad Activa

| Crate | Versión | Advisory | CVSS | Descripción | Solución |
|-------|---------|----------|------|-------------|----------|
| `time` | 0.3.45 | RUSTSEC-2026-0009 | 6.8 | DoS via Stack Exhaustion | `>= 0.3.47` |

**Contexto:** Dependencia transitiva usada por `tauri-plugin-notification`, `tauri-codegen`, `serde_with`, `plist`, `cookie`. Fix trivial:

```bash
cargo update -p time
```

#### Advertencias de Unsoundness (2)

| Crate | Versión | Advisory | Riesgo Práctico |
|-------|---------|----------|-----------------|
| `glib` | 0.18.5 | RUSTSEC-2024-0429 | Bajo — solo Linux, no se usa directamente |
| `lru` | 0.12.5 | RUSTSEC-2026-0002 | Bajo — transitiva via ratatui, IterMut no expuesto |

#### Dependencias No Mantenidas (17)

- **10 crates GTK3** (`atk`, `gdk`, `gtk`, etc.) — Transitivas de Tauri/wry para Linux. Sin solución hasta que Tauri migre a GTK4.
- **5 crates unic-*** — Transitivas via `urlpattern -> tauri-utils`. Sin CVEs conocidos.
- **`fxhash`**, **`paste`**, **`proc-macro-error`** — Transitivas, sin exploits conocidos.

### 1.2 Vulnerabilidades Frontend (bun audit)

| Paquete | Versión | Severidad | Advisory | Contexto |
|---------|---------|-----------|----------|----------|
| `serialize-javascript` | 6.0.2 | HIGH | GHSA-5c6j-r48x-rmvq | Solo en devDependencies (`@wdio/mocha-framework`) — NO incluido en producción |

---

## 2. Criptografía (crypto.rs)

### Hallazgos

#### SEC-03: Sin Zeroización de Claves (CWE-316) — MEDIA

Las funciones `encrypt_bytes`, `decrypt_bytes`, `encrypt_json`, `decrypt_json` reciben `key: &[u8; 32]` pero nunca invocan `zeroize()`. No se encontró la dependencia `zeroize` en `Cargo.toml`.

**Impacto:** Core dumps o ataques cold-boot podrían recuperar claves criptográficas.

**Remediación:**
```toml
# Cargo.toml (core)
zeroize = { version = "1", features = ["derive"] }
```
```rust
use zeroize::Zeroize;

// Usar Zeroizing<> wrapper para claves temporales
let mut key_copy = Zeroizing::new(*key);
let cipher = Aes256Gcm::new_from_slice(&*key_copy)
    .map_err(|e| format!("invalid key: {e}"))?;
// key_copy se zeroiza automáticamente al salir de scope
```

#### SEC-04: Sin Key Derivation Function (CWE-327) — MEDIA

No existe implementación de PBKDF2, Argon2, ni HKDF. Las claves se pasan directamente como `[u8; 32]`.

**Remediación:**
```rust
use hkdf::Hkdf;
use sha2::Sha256;

pub fn derive_key(master_key: &[u8], context: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, master_key);
    let mut okm = [0u8; 32];
    hk.expand(context, &mut okm).expect("HKDF expand failed");
    okm
}
```

#### SEC-17: Comparación SHA-256 sin Tiempo Constante (CWE-208) — BAJA

```rust
// crypto.rs:136 — comparación early-exit
if computed_hash != release_sig.sha256 { ... }
```

**Contexto atenuante:** La verificación Ed25519 posterior (línea 152) usa comparaciones constantes en `ed25519-dalek`. Este check es solo pre-validación local. Riesgo bajo.

**Remediación:**
```toml
subtle = "2"
```
```rust
use subtle::ConstantTimeEq;
if computed_hash.as_bytes().ct_eq(release_sig.sha256.as_bytes()).into() { ... }
```

### Aspectos Correctos

- AES-256-GCM implementado correctamente con nonces CSPRNG (`rand::thread_rng().fill_bytes`)
- Ed25519 (`ed25519-dalek v2`) con verificación usando `trusted_public_key` (no extraído del payload)
- Doble verificación de releases: SHA-256 (integridad) + Ed25519 (autenticidad)
- Sin claves hardcodeadas en producción
- Keyring nativo por plataforma correctamente configurado

---

## 3. MITRE ATT&CK (security.rs)

### Hallazgos

#### SEC-01: Técnica T1043 Obsoleta (CWE-1059) — ALTA

```rust
// security.rs:88-91
MitreTechnique {
    technique_id: "T1043".to_string(),  // REVOCADA en ATT&CK v8 (Oct 2020)
    tactic: "Command and Control".to_string(),
    name: "Commonly Used Port".to_string(),
},
```

**T1043 fue fusionada/revocada.** Cualquier reporte generado con esta técnica será rechazado por herramientas SIEM/SOAR modernas.

**Remediación:**
```rust
MitreTechnique {
    technique_id: "T1071".to_string(),
    tactic: "Command and Control".to_string(),
    name: "Application Layer Protocol".to_string(),
},
```

#### SEC-15: Cobertura MITRE Incompleta — MEDIA

**Técnicas actualmente cubiertas:**
- T1055.001 (DLL Injection)
- T1055.003 (Thread Execution Hijacking)
- T1055.012 (Process Hollowing)
- T1003 (OS Credential Dumping)
- T1574 (Hijack Execution Flow)
- ~~T1043~~ (Obsoleta → T1071)

**Técnicas críticas faltantes:**

| Técnica | Nombre | Prioridad |
|---------|--------|-----------|
| T1059 | Command and Scripting Interpreter | Alta |
| T1071 | Application Layer Protocol | Alta |
| T1053 | Scheduled Task/Job | Media |
| T1547 | Boot/Logon Autostart | Media |
| T1562 | Impair Defenses | Media |

**Remediación:**
```rust
pub enum BehaviorIndicator {
    // ... existentes ...
    SuspiciousScriptExecution,    // T1059
    ApplicationLayerProtocol,     // T1071
    ScheduledTaskCreation,        // T1053
    BootAutostart,                // T1547
    DefenseImpairment,            // T1562
}
```

---

## 4. Checklist NIST SP 800-53

Evaluación contra controles relevantes de NIST SP 800-53 Rev. 5:

### AC — Control de Acceso

| Control | Descripción | Estado | Nota |
|---------|-------------|--------|------|
| AC-3 | Access Enforcement | ✅ Cumple | Rate limiting + blocklists en IPC |
| AC-5 | Separation of Duties | ✅ Cumple | Frontend no ejecuta kills directamente; requiere confirmación |
| AC-6 | Least Privilege | ⚠️ Parcial | Tauri capabilities bien restringidas, pero `apply_ai_rules` sin rate limit (SEC-02) |
| AC-7 | Unsuccessful Logon Attempts | N/A | No aplica (app de escritorio, no web service) |
| AC-17 | Remote Access | ✅ Cumple | Sin acceso remoto; conexiones solo a APIs documentadas |

### AU — Auditoría y Accountability

| Control | Descripción | Estado | Nota |
|---------|-------------|--------|------|
| AU-2 | Event Logging | ✅ Cumple | Heartbeats de seguridad cifrados con AES-256-GCM |
| AU-3 | Content of Audit Records | ✅ Cumple | Snapshots incluyen timestamp, PIDs, métricas, eventos de red |
| AU-8 | Time Stamps | ✅ Cumple | `updated_at_unix_ms` en cada snapshot |
| AU-9 | Protection of Audit Information | ⚠️ Parcial | Audit trail cifrado, pero base CVE sin firma (SEC-12) |
| AU-10 | Non-repudiation | ⚠️ Parcial | Ed25519 disponible pero no integrado en audit trail |

### IA — Identificación y Autenticación

| Control | Descripción | Estado | Nota |
|---------|-------------|--------|------|
| IA-5 | Authenticator Management | ⚠️ Parcial | API keys en keyring nativo, pero fallback inseguro (SEC-09) |
| IA-7 | Cryptographic Module Authentication | ✅ Cumple | AES-256-GCM + Ed25519 con implementaciones auditadas |
| IA-9 | Service Identification | ✅ Cumple | URLs de API hardcodeadas como constantes, TLS obligatorio |

### SC — Protección de Sistema y Comunicaciones

| Control | Descripción | Estado | Nota |
|---------|-------------|--------|------|
| SC-8 | Transmission Confidentiality | ✅ Cumple | HTTPS obligatorio para APIs cloud; CDP local solo en localhost |
| SC-12 | Cryptographic Key Establishment | ❌ No cumple | Sin KDF (SEC-04); claves sin zeroización (SEC-03) |
| SC-13 | Cryptographic Protection | ⚠️ Parcial | AES-256-GCM y Ed25519 correctos, pero sin KDF |
| SC-17 | Public Key Infrastructure | ⚠️ Parcial | Ed25519 infraestructura lista, pero pubkey de updater vacía (SEC-10) |
| SC-23 | Session Authenticity | ✅ Cumple | CSP estricto, sin `unsafe-inline`/`unsafe-eval` |
| SC-28 | Protection of Information at Rest | ⚠️ Parcial | Heartbeats cifrados, pero API keys pueden caer a texto plano (SEC-09) |

### SI — Integridad del Sistema e Información

| Control | Descripción | Estado | Nota |
|---------|-------------|--------|------|
| SI-2 | Flaw Remediation | ⚠️ Parcial | `cargo audit` en CI, pero `continue-on-error: true` (INFO-03) |
| SI-3 | Malicious Code Protection | ✅ Cumple | MITRE ATT&CK mapping + rules engine + behavior detection |
| SI-5 | Security Alerts | ✅ Cumple | Sistema de alertas con stores reactivos |
| SI-7 | Software/Firmware/Info Integrity | ❌ No cumple | Releases sin firma Ed25519 en CI (SEC-14); updater pubkey vacía (SEC-10) |
| SI-10 | Information Input Validation | ⚠️ Parcial | Validación IPC excelente, pero prompt injection débil (SEC-06) |

### Resumen NIST

| Estado | Cantidad | Porcentaje |
|--------|----------|------------|
| ✅ Cumple | 13 | 57% |
| ⚠️ Parcial | 9 | 39% |
| ❌ No cumple | 2 | 9% |
| N/A | 1 | - |

---

## 5. Seguridad IPC

### SEC-02: `apply_ai_rules` sin Rate Limiting (CWE-770) — ALTA

```rust
// lib.rs:392-397 — Sin check_rate_limit
fn apply_ai_rules(payload: String) -> Result<usize, String> {
    macmon_core::rules_engine::upsert_rules_from_ai_json(&payload)
}
```

**Impacto:** Un frontend comprometido podría inyectar/reemplazar reglas sin límite.

**Remediación:**
```rust
fn apply_ai_rules(payload: String) -> Result<usize, String> {
    macmon_core::rate_limit::check_rate_limit(
        "apply_ai_rules",
        &macmon_core::rate_limit::profiles::CONFIG,
    )?;
    if payload.len() > 64 * 1024 {
        return Err("payload exceeds 64KB limit".into());
    }
    macmon_core::rules_engine::upsert_rules_from_ai_json(&payload)
}
```

### SEC-05: Reglas AI Reemplazan Completamente (CWE-862) — MEDIA

`upsert_rules_from_ai_json` ejecuta `guard.rules = payload.rules` — reemplazo total, no merge. Un payload vacío elimina todas las reglas.

**Remediación:** Implementar merge real o mantener reglas base inmutables:
```rust
pub fn upsert_rules_from_ai_json(payload_json: &str) -> Result<usize, String> {
    let payload: AiRulesPayload = serde_json::from_str(payload_json)?;
    let mut guard = state().write().map_err(|_| "lock poisoned".to_string())?;
    for new_rule in payload.rules {
        if let Some(existing) = guard.rules.iter_mut().find(|r| r.id == new_rule.id) {
            *existing = new_rule;
        } else {
            guard.rules.push(new_rule);
        }
    }
    Ok(guard.rules.len())
}
```

### SEC-11: `kill_processes` — 1 Token para N PIDs (CWE-770) — MEDIA

```rust
// lib.rs:316-330 — Un solo check para N kills
fn kill_processes(pids: Vec<u32>) -> Result<KillProcessesResult, String> {
    macmon_core::rate_limit::check_rate_limit("kill_processes", &profiles::KILL)?;
    for pid in pids { ... }  // Sin límite de batch
}
```

**Remediación:**
```rust
fn kill_processes(pids: Vec<u32>) -> Result<KillProcessesResult, String> {
    const MAX_BATCH: usize = 50;
    if pids.len() > MAX_BATCH {
        return Err(format!("batch limited to {} PIDs", MAX_BATCH));
    }
    for _ in 0..pids.len() {
        macmon_core::rate_limit::check_rate_limit("kill_process", &profiles::KILL)?;
    }
    // ... proceso de kill
}
```

### SEC-10: Updater Pubkey Vacío (CWE-347) — MEDIA

```json
// tauri.conf.json
"updater": {
    "pubkey": ""
}
```

Las actualizaciones desde CrabNebula CDN no verifican firma criptográfica. Un MITM podría distribuir updates maliciosos.

**Remediación:** Generar keypair Ed25519 con `tauri signer generate` y configurar la pubkey.

### Configuración CSP — CORRECTA

```
default-src 'self';
style-src 'self';
connect-src 'self' https://openrouter.ai https://api.openai.com
            https://generativelanguage.googleapis.com https://api.anthropic.com;
script-src 'self';
form-action 'none';
frame-ancestors 'none'
```

- Sin `unsafe-inline` ni `unsafe-eval`
- Solo los 4 endpoints de LLM documentados en `connect-src`
- Capabilities de Tauri mínimas y bien configuradas
- Shell restringido a `https://github.com/chochy2001/omnimon/*`
- Store limitado a `$APPDATA/omnimon/*`

---

## 6. Seguridad AI/LLM

### SEC-06: Prompt Injection Trivialmente Evadible (CWE-77/74) — MEDIA

```rust
// ai.rs:27-46 — Lista de 8 frases bloqueadas
fn check_prompt_injection(input: &str) -> Result<(), String> {
    let blocked = ["ignore previous", "act as", "you are now", ...];
    // Bypass trivial con homoglyphs, encodings, sinónimos, idiomas
}
```

**Remediación:** Reemplazar blocklist por un enfoque multi-capa:
1. Normalización Unicode antes de comparar
2. Ampliar cobertura con variantes multiidioma
3. Considerar clasificador ML ligero para evaluación de riesgo
4. Aplicar rate limiting agresivo a inputs sospechosos

### SEC-07: `analyze_with_ai()` sin Protección (CWE-74) — MEDIA

`analyze_with_ai_key()` recibe `processes_json` y `profile` del frontend y los incluye directamente en el prompt sin aplicar `check_prompt_injection()`. Un proceso malicioso podría nombrarse con un payload de prompt injection.

**Remediación:**
```rust
pub async fn analyze_with_ai_key(
    processes_json: &str, profile: &str, ...
) -> Result<String, String> {
    check_prompt_injection(profile)?;
    // ... continuar
}
```

### SEC-08: Tool Calling Permisivo (CWE-20) — MEDIA

`parse_tool_call()` (línea 649) valida que el `tool` sea uno de 5 permitidos, pero no valida la estructura de `args`. `kill_by_name` acepta cualquier string que podría matchear muchos procesos.

**Mitigación existente:** Las acciones destructivas son diferidas (requieren confirmación del usuario). Riesgo reducido pero no eliminado.

### Aspectos Positivos AI

- API keys nunca en logs (verificado: solo se loguean proveedor, modelo, longitudes, status codes)
- TLS obligatorio para todos los proveedores cloud (URLs HTTPS hardcodeadas como constantes)
- Test `ai_provider_api_urls_are_https_or_localhost()` en CI
- Timeout de 60s previene conexiones colgadas
- Retry con backoff exponencial limitado a 1 reintento
- Sin riesgo SSRF (URLs no configurables por usuario)

---

## 7. Seguridad de Red y Navegador

### Aspectos Correctos

- **pcap** en modo NO promiscuo (`promisc(false)`)
- **Sanitización de tabs CDP:** `sanitize_tab_id()` y `sanitize_tab_url()` rechazan control chars, path traversal, esquemas peligrosos (`file://`, `javascript:`, `data:`)
- **AppleScript seguro:** Argumentos via `cmd.arg()` (no interpolados en el script)
- **Aritmética saturante** (`saturating_add/sub`) en contadores de red
- **Watcher resiliente:** `catch_unwind()` previene que pánicos maten el hilo de monitoreo
- **Timeout CDP:** 2 segundos en todas las requests HTTP

### Hallazgos Menores

- **SEC-20:** `lsof`/`netstat` sin ruta absoluta — riesgo de path hijacking si atacante controla `$PATH`
- **SEC-21:** `OMNIMON_EBPF_OBJECT` env var permite cargar programa eBPF arbitrario en el kernel
- **SEC-25:** Funciones CDP públicas aceptan `base_url` arbitrario (mitigado: el frontend usa `NativeTabProvider` que hardcodea localhost)

---

## 8. Integridad de Releases y CI/CD

### SEC-13: GitHub Actions sin Pinnear por SHA (CWE-829) — MEDIA

| Action | Estado | Riesgo |
|--------|--------|--------|
| `actions/checkout@v4` | Tag flotante | Medio |
| `dtolnay/rust-toolchain@stable` | Tag flotante | Medio |
| `Swatinem/rust-cache@v2` | Tag flotante | Medio |
| `oven-sh/setup-bun@v2` | Tag flotante | Medio |
| `taiki-e/install-action@cargo-llvm-cov` | Sin versión | Medio |
| `taiki-e/install-action@cargo-audit` | Sin versión | Medio |
| `actions/upload-artifact@v4` | Tag flotante | Medio |
| `tauri-apps/tauri-action@v0` | Tag flotante | Medio |
| **`trufflesecurity/trufflehog@main`** | **Branch main** | **Alto** |

**Remediación:** Pinnear todas las actions por commit SHA completo:
```yaml
- uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11  # v4.1.1
```

### SEC-14: Releases sin Firma Criptográfica (CWE-347) — MEDIA

El job `sign-release` genera `SHA256SUMS.txt` pero sin firma GPG ni Ed25519. Un atacante con acceso a la release de GitHub podría reemplazar binarios Y checksums.

**Remediación:** Integrar la infraestructura Ed25519 existente en `crypto.rs` en el pipeline de release:
```yaml
- name: Sign checksums
  run: |
    omnimon-cli sign --key "$SIGNING_KEY" SHA256SUMS.txt
    # Genera SHA256SUMS.txt.sig
```

### Propuesta: Endpoint de Verificación de Hash

```
GET https://omnimon.com.mx/api/verify/{version}/{platform}
Response: {
    "sha256": "abc123...",
    "ed25519_signature": "def456...",
    "signed_at": "2026-03-08T00:00:00Z"
}
```

---

## 9. Almacenamiento de Credenciales

### SEC-09: Fallback Inseguro de API Keys (CWE-312) — MEDIA

```rust
// lib.rs:372-382
if macmon_core::ai::save_api_key(ai_provider, &trimmed_key).is_ok() {
    return Ok(());  // Keyring nativo — seguro
}
// Fallback: Tauri Store en texto plano
let store = app.store(STORE_FILENAME).map_err(|e| e.to_string())?;
store.set(ai_provider.keyring_service(), serde_json::Value::String(trimmed_key));
```

**Impacto:** En sistemas sin keyring (headless Linux, sandboxes), API keys quedan en texto plano en disco sin notificación al usuario.

**Remediación (3 opciones, de mejor a peor):**
1. Cifrar el fallback con `crypto::encrypt_json` usando clave derivada del hardware ID
2. Notificar al usuario explícitamente cuando se use el fallback
3. Fallar con error y pedir al usuario que configure el keyring

---

## 10. Controles Positivos Detectados

La auditoría identificó 15 controles de seguridad bien implementados:

| # | Control | Módulo |
|---|---------|--------|
| 1 | API keys en keyring nativo del OS (Keychain/Credential Manager/Secret Service) | ai.rs, lib.rs |
| 2 | TLS obligatorio para APIs cloud (URLs HTTPS constantes) | ai.rs |
| 3 | Rate limiting con Token Bucket configurable en comandos IPC críticos | rate_limit.rs |
| 4 | Blocklist inmutable de procesos del OS con verificación de ruta confiable | killer.rs |
| 5 | Acciones destructivas diferidas — requieren confirmación del usuario | lib.rs |
| 6 | AES-256-GCM con nonces CSPRNG para audit trail | crypto.rs |
| 7 | Ed25519 para verificación de releases | crypto.rs |
| 8 | Sanitización de tab IDs/URLs contra path traversal y esquemas peligrosos | browser.rs |
| 9 | CSP estricto sin `unsafe-inline`/`unsafe-eval` | tauri.conf.json |
| 10 | Capabilities de Tauri mínimas por ventana | capabilities/default.json |
| 11 | `catch_unwind()` en watcher para resiliencia | watcher.rs |
| 12 | Aritmética saturante en contadores de red | network.rs |
| 13 | TruffleHog para detección de secretos en PRs | CI/CD |
| 14 | Sandbox de plugins Lua (256KB, 1MB mem, 150ms timeout) | plugins.rs |
| 15 | Validación runtime exhaustiva de respuestas IPC en frontend | ipc.ts |

---

## 11. Plan de Remediación Priorizado

### Fase 1 — Inmediata (1-2 semanas)

| ID | Acción | Esfuerzo | Impacto |
|----|--------|----------|---------|
| SEC-01 | Reemplazar T1043 por T1071 en security.rs | 10 min | Elimina técnica MITRE inválida |
| SEC-02 | Agregar rate limiting a `apply_ai_rules` | 15 min | Cierra vector de inyección masiva de reglas |
| SEC-10 | Generar y configurar pubkey del updater | 30 min | Habilita verificación de updates |
| SEC-11 | Limitar batch size en `kill_processes` | 15 min | Previene bypass de rate limit |
| DEP-01 | `cargo update -p time` | 5 min | Resuelve RUSTSEC-2026-0009 |

### Fase 2 — Corto plazo (2-4 semanas)

| ID | Acción | Esfuerzo | Impacto |
|----|--------|----------|---------|
| SEC-03 | Agregar `zeroize` para claves criptográficas | 2 hrs | Protección contra memory dumps |
| SEC-04 | Implementar HKDF para derivación de claves | 4 hrs | Cumplimiento SC-12 NIST |
| SEC-05 | Implementar merge real en `upsert_rules_from_ai_json` | 2 hrs | Previene eliminación total de reglas |
| SEC-09 | Cifrar fallback de API keys o eliminar fallback | 3 hrs | Protección de credenciales |
| SEC-13 | Pinnear GitHub Actions por SHA | 1 hr | Supply chain security |
| SEC-14 | Firmar releases con Ed25519 | 4 hrs | Integridad de distribución |
| SEC-16 | Rate limiting en automations/plugins IPC | 30 min | Consistencia de protección |

### Fase 3 — Mediano plazo (1-2 meses)

| ID | Acción | Esfuerzo | Impacto |
|----|--------|----------|---------|
| SEC-06/07 | Mejorar detección de prompt injection | 1-2 días | Defensa AI robusta |
| SEC-15 | Ampliar cobertura MITRE ATT&CK (T1059, T1053, T1547) | 1 día | Detección más completa |
| SEC-12 | Firmar base de datos CVE con Ed25519 | 4 hrs | Integridad de datos de auditoría |
| SEC-19 | Agregar TTL y límite de tamaño al cache AI | 2 hrs | Prevención de memory exhaustion |
| SEC-20 | Usar rutas absolutas para `lsof`/`netstat` | 30 min | Protección contra path hijacking |
| SEC-21 | Verificar integridad de eBPF object o eliminar env var | 2 hrs | Protección del kernel |

---

## 12. Conclusión

OmniMon v6.0.1 demuestra una postura de seguridad **madura y bien pensada** para una aplicación de escritorio. La arquitectura de defensa en profundidad (rate limiting → blocklists → sanitización → validación runtime) es sólida y consistente.

Las principales áreas de mejora son:
1. **Integridad de distribución** — Configurar firma de updates y releases
2. **Gestión de claves** — Agregar KDF y zeroización
3. **Cobertura MITRE** — Actualizar técnicas obsoletas y ampliar detección
4. **Supply chain CI/CD** — Pinnear dependencias de Actions

Ninguna de las vulnerabilidades encontradas es explotable de forma remota sin acceso previo al sistema. El riesgo residual es aceptable para una herramienta de monitoreo de escritorio, y las remediaciones propuestas son incrementales y no requieren rediseño arquitectónico.
