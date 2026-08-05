# Audit Testing - OmniMon

> Estado actual 2026-06-29: este reporte es una captura histórica del
> 2026-03-08. La validación actual pasó `bun run typecheck`, `bun run test`
> (45 archivos / 701 tests), `bun run build`, `bun run test:coverage`
> (85.75% statements, 70.45% branches, 86.64% functions, 86.58% lines),
> `cargo check --workspace`, `cargo clippy --workspace -- -D warnings`, y
> `cargo test --workspace` (458 Rust tests). El workflow actual gatea frontend
> line coverage >=75% y Rust line coverage >=70%, no 85%.

## Alcance

- Worktree: `worktree-gpt-1`
- Branch: `audit/gpt-testing`
- Fecha: 2026-03-08
- Foco: testing, coverage, CI/CD y calidad de codigo

## Estado actual de tests

### Frontend (`v4/apps/desktop`)

- `bun install`: requerido al inicio; sin dependencias instaladas `vitest` no estaba disponible.
- `bun run test`: OK
  - 26 archivos de test
  - 476 tests
  - 0 fallos
  - duracion aproximada: 3.49s
- `bun run build`: OK (`vite build` completo)
- `bunx tsc --noEmit`: OK despues de corregir `SystemStats` en `src/stores/processes.ts`
- Nota: el stderr observado en varios tests corresponde a casos simulados de error IPC/AppleScript/keyring y no representa fallos reales de la suite.

### Backend / Rust (`v4`)

- `cargo fmt --check`: OK
- `cargo test --workspace`: OK
  - `cli`: 4 tests OK
  - `core` unitarios: 124 tests OK
  - `core` integracion: 91 tests OK
  - `tui`: 4 tests OK
  - total ejecutado: 223 tests OK
- `cargo clippy --workspace -- -D warnings`: OK

## Coverage actual vs target (85%)

### Frontend global

- Statements: 71.35% (`3958/5547`) - FAIL vs 85%
- Functions: 75.00% (`987/1316`) - FAIL vs 85%
- Branches: 58.10% (`1349/2322`) - FAIL vs 85%

### Frontend prioritario

- `v4/apps/desktop/src/stores/processes.ts`
  - Statements: 96.55%
  - Functions: 100.00%
  - Branches: 86.78%
- `v4/apps/desktop/src/lib/ipc.ts`
  - Statements: 96.60%
  - Functions: 100.00%
  - Branches: 84.67% - cerca del objetivo, pero aun debajo

### Rust `core` global (`cargo llvm-cov -p core --lib --summary-only --json`)

- Lines: 71.63% (`3328/4646`) - FAIL vs 85%
- Functions: 67.42% (`360/534`) - FAIL vs 85%
- Regions: 71.18% (`4944/6946`) - FAIL vs 85%

### Rust prioritario

- `v4/crates/core/src/watcher.rs`
  - Lines: 94.92%
  - Functions: 89.47%
  - Regions: 95.74%
- `v4/crates/core/src/killer.rs`
  - Lines: 77.09%
  - Functions: 91.30%
  - Regions: 78.76%
- `v4/crates/core/src/network.rs`
  - Lines: 97.08%
  - Functions: 96.30%
  - Regions: 98.00%
- `v4/crates/core/src/security.rs`
  - Lines: 95.35%
  - Functions: 100.00%
  - Regions: 94.68%

## Tests nuevos generados

### Frontend

- `v4/apps/desktop/src/lib/__tests__/ipc.test.ts`
  - cobertura nueva para `ipcFocusBrowserTab`
  - cobertura nueva para `ipcAnalyzeContext`
  - cobertura nueva para `ipcCheckApiKey`
  - cobertura nueva para `ipcGetWindowVisible`
  - cobertura nueva para `ipcApplyAiRules`
  - cobertura nueva para `ipcAiChat`
  - cobertura nueva para `ipcGetAiRulesSchema`
  - cobertura nueva para `ipcGetNetworkData`
  - cobertura nueva para `ipcListPlugins`
  - cobertura nueva para `ipcInstallPlugin`
  - cobertura nueva para `ipcSetPluginEnabled`
  - cobertura nueva para `ipcRemovePlugin`
  - validacion negativa adicional de payloads plugin/chat/network
- `v4/apps/desktop/src/stores/__tests__/processes.test.ts`
  - cobertura nueva para desactivar polling de browser tabs
  - cobertura nueva para activar polling de red de forma condicional

### Rust

- `v4/crates/core/src/watcher.rs`
  - test para snapshot inicial con campos de red vacios y backend `Unknown`
- `v4/crates/core/src/killer.rs`
  - test de mensajes `Display` para `KillError`
- `v4/crates/core/src/network.rs`
  - test de `CollectorWindow::merge_from`
  - test de `CollectorWindow::into_rates`
  - test de parseo UDP IPv4 sin cabecera Ethernet
- `v4/crates/core/src/security.rs`
  - test de eventos de red que no disparan politica
  - test de contexto cuando coinciden IP bloqueada + puerto inusual
  - test de confidence reducida cuando falta `detail`

## Calidad de codigo encontrada

- `v4/apps/desktop/src/stores/processes.ts`
  - `shallowEqualStats(...)` usaba campos obsoletos (`total_ram_mb`, `used_ram_mb`, `idle_processes`)
  - esto rompia `bunx tsc --noEmit`
  - corregido para usar `ram_total_gb`, `ram_used_pct`, `swap_used_mb`, `total_processes`
- Coverage bajo y desigual en frontend visual
  - componentes sin cobertura o casi nula: `Automations.svelte`, `CloudSync.svelte`, `Plugins.svelte`, `SmartAlerts.svelte`, `ConfirmDialog.svelte`, `AIChat.svelte`, `SystemMetricModal.svelte`
- Coverage bajo en Rust fuera de los modulos prioritarios
  - especialmente `ai.rs`, `audit_trail.rs`, `cloud.rs`, `audit.rs`, `metrics.rs`, `rules_engine.rs`
- Pipeline actual impone un gate real de coverage frontend
  - `.github/workflows/omnimon-ci.yml` corre `bun run test:coverage` y falla si
    la cobertura de líneas cae por debajo de 75%
- Matrix actual incompleta para calidad real multiplataforma
  - Linux corre frontend y coverage
  - macOS no corre frontend
  - Windows solo corre `cargo test -p core --lib`

## Tests faltantes / fragiles detectados

### Faltantes prioritarios

- Frontend:
  - tests directos para `ConfirmDialog.svelte`
  - tests de `CloudSync.svelte`, `Plugins.svelte`, `Automations.svelte`, `SmartAlerts.svelte`
  - 1-2 tests mas de ramas negativas en `ipc.ts` para superar 85% de branches
- Rust:
  - `killer.rs`: escenarios extra de PID reuse / force kill path para subir lines/regions por encima de 85%
  - `metrics.rs`: casos de borde de agregacion/telemetria
  - `audit.rs` y `audit_trail.rs`: mas casos de persistencia/rotacion/error handling
  - `ai.rs`: errores de parsing, retries y payloads inesperados

### Fragilidad / ruido

- Varias pruebas frontend dependen de logs de error esperados; no fallan, pero ensucian CI y pueden ocultar errores reales.
- El fallback de `refreshNetworkConnections` en `processes.test.ts` genera stderr esperado por payload incompleto de `get_network_data`.

## CI/CD actual: hallazgos

- Archivo revisado: `.github/workflows/omnimon-ci.yml`
- Bueno:
  - lint Rust separado
  - cache de Rust
  - coverage backend en Linux con `cargo llvm-cov`
  - secret scanning en PR
  - release multiplataforma separado
- Debilidades:
  - frontend solo se valida en Linux
  - Windows no prueba workspace completo
- `security` ahora falla ante vulnerabilidades reales de `cargo audit`
- el workflow ejecuta audit frontend con `bun audit` cuando está disponible y
  cae a `audit-ci` como fallback
  - no existe SAST de codigo/CodeQL
- no hay gate explicito de coverage frontend >= 85%; el gate vigente es 75%
  - no hay fuzzing automatizado para parsers de input/red/IPC

## Mejoras propuestas de CI/CD (yaml)

### 1) Gate de coverage frontend

```yaml
- name: Run Frontend Tests with Coverage
  run: bun run test:coverage
  working-directory: ./v4/apps/desktop

- name: Enforce Frontend Coverage Gate
  run: |
    node -e '
      const fs = require("fs");
      const data = JSON.parse(fs.readFileSync("./v4/apps/desktop/.coverage-tmp/coverage-summary.json", "utf8"));
      const totals = data.total;
      const threshold = 85;
      for (const key of ["lines", "functions", "statements", "branches"]) {
        if (totals[key].pct < threshold) {
          console.error(`Coverage ${key} below ${threshold}%: ${totals[key].pct}`);
          process.exit(1);
        }
      }
    '
```

### 2) `bun audit` y endurecer `cargo audit`

```yaml
security:
  name: Security Audit
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: oven-sh/setup-bun@v2
    - uses: taiki-e/install-action@cargo-audit
    - name: Bun Audit
      run: bun audit
      working-directory: ./v4/apps/desktop
    - name: Cargo Audit
      run: cargo audit
      working-directory: ./v4
```

### 3) SAST con CodeQL

```yaml
name: CodeQL
on:
  pull_request:
  push:
    branches: [main]

jobs:
  analyze:
    runs-on: ubuntu-latest
    permissions:
      security-events: write
      contents: read
    strategy:
      matrix:
        language: ["javascript-typescript", "rust"]
    steps:
      - uses: actions/checkout@v4
      - uses: github/codeql-action/init@v3
        with:
          languages: ${{ matrix.language }}
      - uses: github/codeql-action/autobuild@v3
      - uses: github/codeql-action/analyze@v3
```

### 4) Matrix mas realista

```yaml
strategy:
  fail-fast: false
  matrix:
    include:
      - platform: linux
        os: ubuntu-latest
        run_frontend: true
        run_workspace_tests: true
      - platform: macos
        os: macos-latest
        run_frontend: true
        run_workspace_tests: true
      - platform: windows
        os: windows-latest
        run_frontend: false
        run_workspace_tests: true
```

### 5) Fuzzing ligero para parsers

```yaml
fuzz:
  name: Fuzz Parsers
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - name: Install cargo-fuzz
      run: cargo install cargo-fuzz
    - name: Run short fuzz smoke
      run: cargo fuzz run parse_ipv4_transport -- -max_total_time=60
      working-directory: ./v4/crates/core
```

## Recomendacion priorizada

1. Convertir `ipc.ts` branch coverage de 84.67% a >= 85% con 2-3 tests negativos mas.
2. Subir `killer.rs` por encima de 85% en lines/regions con pruebas de `identity_matches` y rutas de force-kill.
3. Agregar gates reales de coverage frontend y remover `continue-on-error` en auditorias de seguridad.
4. Extender la matrix para ejecutar frontend tambien en macOS y pruebas backend completas en Windows.
5. Atacar el gran bloque de componentes Svelte sin cobertura, porque hoy arrastran el coverage global muy por debajo del objetivo.
