# OmniMon 6.7.0 — Release Handoff

Documento de cierre de la sesión: qué quedó listo, qué falta para publicar 6.7.0 y dónde se documentó cada cosa. Borrar o archivar este archivo una vez que el tag `v6.7.0` esté publicado en GitHub Releases.

## Estado general

- **PR #3** (`feat: zombie killer + AI safety hardening`) **MERGED** en `main` vía rebase el 2026-04-17.
- **Main commit HEAD:** `4a42bf6` (antes del bump de versión documentado aquí).
- **Nueva versión:** `6.7.0` pinneada en los tres manifiestos, pendiente de commit + push + tag.
- **Landing:** posts EN/ES creados en el repo separado `chochy2001/omnimon_landing`, pendiente de commit + push.
- **Tag `v6.7.0`:** NO creado en esta sesión. El tag dispara el pipeline de release (builds firmados + upload a GitHub Releases) y requiere tu decisión explícita para evitar publicar binarios fuera de tiempo.

## Lo que quedó listo en esta sesión

### Código (commits en `main` del repo `omnimon`)

| SHA | Tipo | Resumen |
|-----|------|---------|
| `9eb01a2` | feat | `zombie-killer`: motor stateless + stateful Tauri, modal, 5 IPC, 25 tests Rust + 7 Vitest |
| `a169c83` | refactor! | `network-alerts`: `EvaluatorState` pasa a ser `pub` y `&mut`, sin singleton (breaking para consumidores out-of-tree) |
| `e73d590` | feat | `ai-safety`: privacy mode, daily budget, keyring delete-first, CSP Ollama, confirmación frontend de automation rules, DPI badge |
| `3b1a183` | chore | clippy 1.95 (`unnecessary_sort_by`, `collapsible_match`) en 10 call sites preexistentes |
| `3cfb0f9` | test | coverage de branches restaurada a 70.37–70.80% (21 tests nuevos en ProfileSettings, Automations, ZombieKiller, `validateAiRule`) |
| `4a42bf6` | docs | CHANGELOG con entrada "Unreleased" para 6.7.0 |

### Documentación (pendiente de commit — ver más abajo)

| Archivo | Cambio |
|---------|--------|
| `v4/Cargo.toml` | `workspace.package.version` 6.6.6 → 6.7.0 |
| `v4/apps/desktop/package.json` | version 6.6.6 → 6.7.0 |
| `v4/apps/desktop/src-tauri/tauri.conf.json` | version 6.6.6 → 6.7.0 |
| `v4/Cargo.lock` | regenerado por `cargo check` |
| `README.md` | badge y alt-text bumped 6.5.0 → 6.7.0 |
| `CHANGELOG.md` | heading promovido `Unreleased` → `6.7.0 (2026-04-17)` |
| `RELEASE_NOTES.md` | reescrito para 6.7.0 (antes mostraba 6.3.0) |
| `omnimon_landing/src/pages/blog/v6-7-0-release.astro` | post EN nuevo |
| `omnimon_landing/src/pages/es/blog/v6-7-0-release-es.astro` | post ES nuevo |
| `omnimon_landing/src/pages/blog/index.astro` | entrada añadida al tope |
| `omnimon_landing/src/pages/es/blog/index.astro` | entrada añadida al tope |
| `RELEASE_6_7_0_HANDOFF.md` | este documento |

### Validaciones locales ya corridas

- `cargo +1.95 check --workspace` → limpio (`omnimon-desktop v6.7.0` compila).
- `cargo +1.95 clippy --workspace --all-targets -- -D warnings` → limpio (antes de los cambios de docs).
- `cargo +1.95 test --workspace` → 458 tests passing (288 core + 95 integration + 53 tauri + 18 + 4 tui).
- `bun run test:coverage` → 689 Vitest, branches 70.37–70.80% (estable sobre el umbral 70%).
- `bun run build` en landing → 12 páginas, incluyendo las dos nuevas v6.7.0.

## Lo que falta para cerrar 6.7.0

### 1. Commit + push del bump de versión y docs (repo `omnimon`)

```bash
cd /Users/jorge/Documents/Apps/omnimon_apps/macmon
git add v4/Cargo.toml v4/Cargo.lock \
        v4/apps/desktop/package.json \
        v4/apps/desktop/src-tauri/tauri.conf.json \
        README.md CHANGELOG.md RELEASE_NOTES.md \
        RELEASE_6_7_0_HANDOFF.md
git commit -m "chore: bump version to v6.7.0"
git push origin main
```

> Este push dispara CI de `main` (no build de release — ese necesita el tag).

### 2. Commit + push del landing (repo `omnimon_landing`)

```bash
cd /Users/jorge/Documents/Apps/omnimon_apps/macmon/omnimon_landing
git add src/pages/blog/v6-7-0-release.astro \
        src/pages/blog/index.astro \
        src/pages/es/blog/v6-7-0-release-es.astro \
        src/pages/es/blog/index.astro
git commit -m "docs(blog): add v6.7.0 release post (EN + ES)"
git push origin main
```

### 3. Tag `v6.7.0` (dispara el build de release)

```bash
cd /Users/jorge/Documents/Apps/omnimon_apps/macmon
git tag -a v6.7.0 -m "OmniMon v6.7.0"
git push origin v6.7.0
```

El workflow `omnimon-ci.yml` se dispara en `push: tags: [ "v*" ]`. Los jobs que corren cuando el tag llega:

- `Lint (fmt + clippy)`
- `Test (linux/macos/windows)`
- `Coverage Gates`
- `Security Audit`
- **`Release Builder`** — genera `.dmg`, `.msi`, `.deb`, `.AppImage`, `.rpm`
- **`Sign Release Artifacts (SHA-256 + Ed25519)`** — firma los binarios
- **`Rename Release Assets`** — los deja con el naming esperado

### 4. Regenerar `v4/releases.json`

El archivo hoy sigue en `6.0.1` con placeholders. El pipeline (`omnimon_release_manifest`) debería regenerarlo con los SHA-256 y firmas reales una vez que el tag buildee. Verificar después del tag:

```bash
cd /Users/jorge/Documents/Apps/omnimon_apps/macmon
cat v4/releases.json | jq '.version'
# → "6.7.0"
```

Si sigue en 6.0.1, correr manualmente el generador (`scripts/sign-release.sh` o equivalente) y hacer un commit aparte.

### 5. Release en GitHub + tap Homebrew

Después de que el pipeline suba artefactos:

- Revisar `https://github.com/chochy2001/omnimon/releases/tag/v6.7.0` (el release builder normalmente crea el draft; publicarlo).
- Actualizar la fórmula de Homebrew en `chochy2001/homebrew-omnimon` apuntando al nuevo DMG y SHA-256 (procedimiento histórico del tap — no tocado en esta sesión).

### 6. Smoke test post-release

```bash
# Verificar que el binario reporta la versión correcta
omnimon --version           # debe decir 6.7.0

# Abrir la app de escritorio y comprobar:
# - Cmd/Ctrl+Shift+Z abre ZombieKiller
# - ProfileSettings muestra la sección "AI Privacy & Budget"
# - StatusBar muestra el badge DPI cuando aplica
```

## Decisiones conscientes que quedaron sin tocar

- **No se creó el tag `v6.7.0`.** Es la única acción que dispara publicación real de binarios; dejarla para una acción explícita tuya.
- **No se actualizó Homebrew tap.** Vive en otro repo y necesita el SHA-256 final de los DMG que sólo se conocen después del build con tag.
- **No se actualizó `v4/releases.json`.** El archivo es regenerado por el release pipeline; tocarlo manualmente introduciría placeholders que luego sobreescribirían valores reales.
- **Sysinfo sigue en 0.30.13.** Plan de upgrade a 0.38 (con su MSRV 1.88, `Process::name -> OsString` y la eliminación de `global_cpu_info`) queda como deuda técnica diferida, documentada en el hilo del PR.
- **Landing `omnimon_apps/omnimon_landing/` (el externo, scaffolding vacío) sigue sin contenido.** El único landing real vive dentro del repo macmon y ya está actualizado.

## Pendientes más amplios / roadmap corto

Sacados de los hilos de revisión del PR #3 y del trabajo de auditoría previo:

- **Copilot review (advisory) dejó 9 comentarios** en el PR #3. Valdría la pena revisarlos para un follow-up; ninguno bloquea merge pero pueden ser mejoras válidas.
- **Traducir las cadenas del Zombie Killer modal a locales adicionales** si se suma otro idioma más allá de EN/ES.
- **Medir el impacto real del `privacy mode` en calidad del LLM.** La redacción es estable (mismo input → mismo token), pero la pérdida de contexto concreto puede hacer peor el razonamiento en queries específicas. Plan: A/B con preguntas repetidas de usuarios piloto.
- **Hacer el `aiDailyLimit` persistente tras reinicio del proceso.** Hoy el `DailyBucket` vive en memoria y se resetea en cada cold start; si el proceso se reinicia el mismo día, recupera los 200 tokens completos. Limitación conocida ya declarada en la PR description.

## Cómo reanudar la próxima sesión

1. Abrir este archivo.
2. Ejecutar los bloques de comandos en las secciones 1 → 2 → 3 en orden.
3. Verificar CI verde tras cada push.
4. Ejecutar el smoke test de la sección 6.
5. Borrar `RELEASE_6_7_0_HANDOFF.md` una vez confirmado el release.
