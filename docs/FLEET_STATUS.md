# OmniMon (macmon) — Fleet Status

Verified 2026-10-14. Rust/Tauri desktop app + Go-free stack; manifests at `6.8.0`, no tag/release for 6.7.0/6.8.0.

## Revisions

| Ref | SHA | Date |
|---|---|---|
| HEAD (`omnimon-RUSTAUDIT`) | `7b909a4` | 2026-10-14 |
| `origin/main` | `2c4ab80` | 2026-10-14 |
| Latest tag | `v6.6.6` (no `v6.7`/`v6.8` tag) | — |

## Prod / live

- Prod containers on `web-app-proxy`: none (`docker ps | grep -iE 'omni|macmon'` → no match). N/A by design: desktop app.
- Live landing `https://omnimon.com.mx`: HTTP/2 200, static (Cloudflare), `last-modified 2026-10-11`. No revision endpoint; landing is the separate `omnimon_landing` repo (git-ignored here).

## Gates (measured 2026-10-14, this tree)

| Command | Result |
|---|---|
| `cargo fmt --check` (v4/) | PASS exit 0 |
| `cargo check --workspace` | PASS (after tui fix below) |
| `cargo clippy --workspace` | PASS, 0 warnings |
| `cargo test -p core` | 337 + 97 pass, 0 fail |
| `cargo test -p omnimon-tui` | 33 pass, 0 fail (171 s) |
| `cargo test --workspace` | BLOCKED: `cli` tests hang in `SecKeychainFindGenericPassword` mutex (pre-existing, see pendings) |
| `cargo run -p cli -- --help` | exit 0 (was exit 101 before tui fix) |
| desktop `vitest run` | 715/715, 47 files |
| landing `bun test` | 63 pass / 0 fail (prior viewport failure gone; fixed outside this lane) |
| `curl -sI https://omnimon.com.mx` | HTTP/2 200 |

## Fixes applied 2026-10-14 (this lane)

- `v4/crates/tui/src/event.rs:22`: fixed E0277 (`Terminal::draw` returns `B::Error` under ratatui 0.30) by mapping to `io::Error`. Unblocks `tui`, `cli`, workspace check/clippy. Version-agnostic; `origin/main` also requires ratatui 0.30.
- `CHANGELOG.md`: 6.8.0/6.7.0 headings marked unreleased, no tag (latest `v6.6.6`).
- `docs/ROADMAP.md`: items 2 (temporal correlation) and 3 (Playwright E2E) marked DONE with evidence; item 4 corrected to real pipeline `.github/workflows/omnimon-ci.yml` (no `notarytool` step exists; only Ed25519/SHA-256 + Tauri signing).
- `RELEASE_6_7_0_HANDOFF.md`: staleness header (historical 2026-04-17 doc; tag still unpublished).

## Real pendings

LOCAL (agent-actionable):
1. RESOLVED 2026-10-22 (pendientes wave): keychain/network tests are hermetic
   (`hermetic_keychain` mock in cli/tui); full cli+tui suites finish in seconds.
2. RESOLVED 2026-10-22: CloudSync upload IPC `sync_cloud` implemented
   (encrypted telemetry payload; clean error without key). Real CrabNebula
   endpoint/contract still OWNER-unverified.
3. MEASURED 2026-10-25: `cargo llvm-cov --workspace` lines 86.56%
   (16,367 total lines), functions 82.08%; `cargo audit` 0 vulnerabilities
   (5 allowed warnings: unmaintained gtk stack + 1 unsound).
4. RESOLVED 2026-10-22: Playwright E2E job in CI; orphan network IPC commands
   removed (event + client filter retained); stores tsc errors fixed.
5. Landing screenshot: AVIF/WebP generated (-83%/-72%), PNG kept as fallback.

OWNER-ONLY:
1. Publish `v6.7.0`/`v6.8.0` tags + Universal DMG (triggers signed release pipeline; explicit decision per handoff doc). Homebrew cask stays on `6.6.6` until then.
2. Apple Developer secrets for notarization (ROADMAP item 4); pipeline has no `notarytool` step.

## Barrido 2026-10-16 lane E (cierre sesion, sin commit/push/merge/delete/deploy)

- Rama `omnimon-RUSTAUDIT` `7b909a4` (2026-10-14); arbol: 9 M
  (`AGENTS.md`, `CHANGELOG.md`, `RELEASE_6_7_0_HANDOFF.md`, `docs/`,
  `v4/` schemas + `tui/event.rs`) + `?? docs/FLEET_STATUS.md`,
  `git diff --check` limpio. Desktop N/A prod (sin contenedores, by design).
- Landing separada: `https://omnimon.com.mx` 200 hoy (ver repo
  `omnimon_landing` para huella).

## Inventario docs-pendientes 2026-10-22 (solo contraste doc vs código, sin gates nuevos)

Evidencia medida hoy: `v4/Cargo.toml:11` + desktop `package.json`/`tauri.conf.json`
= `6.8.0`; `git tag` último = `v6.6.6`; cask `distribution/homebrew/omnimon.rb`
= `6.6.6` (correcto hasta que exista DMG 6.8.0); rama local `omnimon-RUSTAUDIT`
`7b909a4` detrás de `origin/main` `5a271fb` (no re-medido: gates de arriba son
de 2026-10-14, no de hoy).

Corregido hoy (stale probado):
- `docs/ARCHITECTURE.md:3`, `docs/CLI_MANUAL.md:1`, `COMMANDS_REFERENCE.md:3`:
  cabecera `6.3.0` → `6.8.0` (sin tag, último `v6.6.6`).
- `AGENTS.md`: landing descrita como anidada con git propio
  (`omnimon_landing/.git` → `chochy2001/omnimon_landing`), no "en este repo".
- `docs/ROADMAP.md` ítem 3 EN/ES: `DONE/HECHO` → `PARTIAL` (5 specs en
  `v4/apps/desktop/e2e/` existen; ningún job CI los corre, como ya decía su
  segunda línea de estado).
- `AUDIT_DOCS.md`: banner histórico actualizado a `6.8.0`.

Pendientes reales ordenados:
1. P1 OWNER: publicar tag `v6.7.0`/`v6.8.0` + DMG Universal; hasta entonces
   Homebrew queda en `6.6.6` y `RELEASE_6_7_0_HANDOFF.md` sigue histórico.
2. RESUELTO 2026-10-22: CloudSync IPC de subida implementado; pendiente solo
   la confirmación OWNER del endpoint real CrabNebula.
3. RESUELTO 2026-10-22: job CI Playwright E2E añadido (flaky conocido:
   `e2e/ai-chat.spec.ts` pasa en retry).
4. P2 OWNER: ROADMAP ítem 4 — secretos Apple Developer + paso `notarytool`
   (ausente en `.github/workflows/`, verificado por grep).
5. RESUELTO 2026-10-22: tests keychain/red herméticos; quedan residuales
   menores (2 errores tsc fuera de stores, 64 de `astro check` en landing).
6. P3 DOC: `v4/docs/ROADMAP.md` y `v4/docs/PARITY_GAPS.md` son históricos
   (v4.1–v4.6, mencionan proveedor OpenAI directo ya retirado en
   `CHANGELOG.md` Unreleased); tratar como archivo, no como roadmap vigente.
   El roadmap vigente es `docs/ROADMAP.md`.
7. P3 DOC: contradicción `CLAUDE.md` (consume `capdesis-ui`) vs `AGENTS.md`
   (no requerido, no añadir a desktop Svelte/Tauri; 0 referencias en
   `package.json` de desktop y landing). Requiere decisión del operador;
   no resuelta aquí.

Cerrados con evidencia (no reabrir): ROADMAP ítem 2 correlación temporal
(`v4/crates/core/src/rules_engine.rs:31` `TemporalCorrelation`); retiro de
OpenAI directo (`CHANGELOG.md` Unreleased + `bf82e3e`).
