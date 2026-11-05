# OmniMon (macmon) — Fleet Status

**Current status, re-checked 2026-11-05 (UTC).** One desktop product (Rust crates `core`, `cli`, `tui`, and the Tauri host). There is no Free app and no Pro app. `CloudTier` is a cloud-sync entitlement, not a second binary.

| Ref | SHA | Evidence |
|---|---|---|
| Desktop `origin/main` | `3471010ca03478bb058bd6234a4fccd5a7a099a2` | Local `main` matches `origin/main`. The only remote head is `main`. |
| Published release | tag `v6.8.0` at `3b859b8` (published 2026-10-26) | `gh release list --repo CAPDESIS/omnimon` marks it Latest. There is no `v6.7.0` tag. Commits after `3b859b8` (including the Spanish UI copy) are not in that binary. |
| Landing `origin/main` and live site | `830192fb1be54b315e05a23ec7b744c9d3737427` | `curl` of `https://omnimon.com.mx/version.json` and the `omnimon-build` meta, `built_at` `2026-11-03T02:59:37Z`. Same SHA as `chochy2001/omnimon_landing` `origin/main`. |
| Desktop CI on that desktop SHA | run `41895931744` success | Linux tests, Playwright E2E, coverage, lint, and the security audit. Release and signing jobs were skipped because there was no tag. macOS and Windows tests run only on `workflow_dispatch`. |

Still open, and not closed by this note: the real CrabNebula endpoint is owner-unverified, Apple notarization (`notarytool`) is absent, and notification click actions in `v4/apps/desktop/src-tauri/src/lib.rs` are comments only (the alerts are already emitted). Local `cargo test` and `bun test` for this checkout were still running when this banner was written, so they are not cited as a pass.

The 2026-10-30 release re-check below remains valid for the published tag. Everything under the historical headings is a snapshot. A line is current only when this banner, or the line itself, says so.

**Release, re-checked 2026-10-30:** `v6.8.0` is published and marked Latest
(`gh release list --repo CAPDESIS/omnimon`: "OmniMon v6.8.0 Latest v6.8.0
2026-10-26T02:25:33Z"; tag commit `3b859b8`, 2026-10-25). It carries 12 assets,
including `OmniMon-6.8.0-macOS-Universal.dmg`, Linux AppImage/deb/rpm, the
Windows MSI, their `.sig.json` files, `SHA256SUMS.txt` and `releases.json`. The
Homebrew cask `distribution/homebrew/omnimon.rb` on `origin/main` is at
`version "6.8.0"`. There is no `v6.7.0` tag or release: 6.7.0 shipped only as a
CHANGELOG heading.

## Revisions (historical, 2026-10-14)

| Ref | SHA | Date |
|---|---|---|
| HEAD (`omnimon-RUSTAUDIT`) | `7b909a4` | 2026-10-14 |
| `origin/main` | `2c4ab80` | 2026-10-14 |
| Latest tag | `v6.8.0` (re-checked 2026-10-30; was `v6.6.6` on 2026-10-14) | 2026-10-25 |

## Prod / live

- Prod containers on `web-app-proxy`: none (`docker ps | grep -iE 'omni|macmon'` → no match). N/A by design: desktop app.
- Landing revision endpoint rechecked 2026-11-04:
  `curl -sS --max-time 15 "https://omnimon.com.mx/version.json?probe=$RANDOM"`
  returns `sha: 830192fb1be54b315e05a23ec7b744c9d3737427`,
  `built_at: 2026-11-03T02:59:37Z`. The former "No revision endpoint" claim
  is stale. The landing is the separate `omnimon_landing` repo (git-ignored
  here); this SHA must be compared to that repository, not the desktop main.
  The 2026-11-04 sentence above is the measurement. The older "NO MEDIBLE"
  label applied only to a desktop-only audit that did not read `/version.json`.
  The 2026-11-05 banner is the current parity: served SHA equals landing `main`.

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

The BLOCKED workspace row is the 2026-10-14 measurement. Hermetic keychain tests superseded it on 2026-10-22. It is not the current result.

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
1. RESOLVED (re-checked 2026-10-30): `v6.8.0` tag and release with the Universal
   DMG exist (2026-10-26) and the Homebrew cask is at `6.8.0`. `v6.7.0` was
   never tagged; it is superseded by `v6.8.0`, not pending.
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
1. Histórico: resuelto el 2026-10-26. `v6.8.0` está publicado (tag `3b859b8`) y el cask está en `6.8.0`. `v6.7.0` no tuvo tag. No reabrir esta fila como pendiente.
2. RESUELTO 2026-10-22: CloudSync IPC de subida implementado; pendiente solo
   la confirmación OWNER del endpoint real CrabNebula.
3. RESUELTO 2026-10-22: job CI Playwright E2E añadido (flaky conocido:
   `e2e/ai-chat.spec.ts` pasa en retry).
4. P2 OWNER: ROADMAP ítem 4 — secretos Apple Developer + paso `notarytool`
   (ausente en `.github/workflows/`, verificado por grep).
5. RESUELTO 2026-10-22: tests keychain/red herméticos. Los residuales citados ese día (2 errores tsc fuera de stores, 64 de `astro check` en landing) no están revalidados en el banner del 2026-11-05.
6. P3 DOC: `v4/docs/ROADMAP.md` y `v4/docs/PARITY_GAPS.md` son históricos
   (v4.1–v4.6, mencionan proveedor OpenAI directo ya retirado en
   `CHANGELOG.md` Unreleased); tratar como archivo, no como roadmap vigente.
   El roadmap vigente es `docs/ROADMAP.md`.
7. RESUELTO 2026-10-25: `CLAUDE.md` y `AGENTS.md` coinciden. `capdesis-ui` no es requerido en el escritorio Svelte/Tauri ni en la landing. No añadirlo salvo una decisión nueva de producto.

Cerrados con evidencia (no reabrir): ROADMAP ítem 2 correlación temporal
(`v4/crates/core/src/rules_engine.rs:31` `TemporalCorrelation`); retiro de
OpenAI directo (`CHANGELOG.md` Unreleased + `bf82e3e`).
