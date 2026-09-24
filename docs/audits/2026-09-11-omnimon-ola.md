> SUPERSEDED 2026-09-23: Paddle y Polar se retiraron por completo el 2026-09-23. Stripe es el unico proveedor de cobro web. Este documento se conserva como historia; no sigas sus instrucciones de Paddle/Polar.

# OmniMon ola 2026-09-11

Repo app: `omnimon_apps/macmon` (`chochy2001/omnimon`)
Landing sibling: `omnimon_apps/macmon/omnimon_landing` (`chochy2001/omnimon_landing`, git aparte, gitignored en macmon)
Fecha: 2026-09-11

No Polar. No deploy flotilla. No se borro `target/`. No `--force`.
No se afirma “6.7.0 prod Mac de todos”.

## KEEP implementados

| Track | Hallazgo | Cambio | Evidencia |
| --- | --- | --- | --- |
| C Lua | `Lua::new()` = mlua `StdLib::ALL_SAFE` con `io`/`os`/`package`; docs decian “sandbox” | `create_plugin_lua()`: stdlib `coroutine/table/string/utf8/math`; nil `load/loadfile/dofile/require/io/os/package/debug`; tests | `v4/apps/desktop/src-tauri/src/plugins.rs`; `cargo test -p omnimon-desktop --lib` filtros plugin |
| D Landing | CookieBanner huerfano; no existian `/privacy` `/terms` | Paginas EN `/privacy` `/terms`, ES `/es/privacy` `/es/terms`; banner montado en `Layout`; href reales | `omnimon_landing` `bun run test` + `bun run build` |
| B Front | `table.emptyTitle`/`emptyDesc` ausentes; copy EN “sandboxed” | i18n EN/ES + test empty state; subtitle sin sandbox | `ProcessTable.test.ts`, `Plugins.test.ts` |
| E CVE/CI | `CVE_REPORT.md` decia `time 0.3.45` | Lockfile `time 0.3.54`; reportes alineados. CI cargo+bun ya existia (app y landing aparte) | `v4/Cargo.lock`; `.github/workflows/omnimon-ci.yml`; landing `.github/workflows/ci.yml` |
| F Homebrew | Formula `6.6.6` vs workspace `6.7.0` | Se **deja** `6.6.6` (ultimo tag publicado) + comentario; no SHA falso de 6.7.0 | `distribution/homebrew/omnimon.rb` |
| A Rust | Tests crate acotados | `cargo test -p core -p cli --offline` (no `--workspace` local) | comandos abajo |

## Fuera de alcance

- Tag `v6.7.0` / DMG universal / bump Homebrew
- Polar, Sentry dual, capdesis-ui en desktop
- `cargo test --workspace` completo (Tauri + capture; CI ya lo corre)

## Validacion (ejecutada)

| Comando | Resultado |
| --- | --- |
| `cd v4 && cargo fmt -p omnimon-desktop && cargo clippy -p omnimon-desktop --lib --offline -- -D warnings` | exit 0 |
| `cd v4 && cargo test -p omnimon-desktop --offline --lib run_plugin_source -- --test-threads=1` | 5 passed (incluye stdlib restringida) |
| `cd v4 && cargo test -p core -p cli --offline -- --test-threads=1` | cli unit 48, cli_tests 18, core unit 321, core_integration 96, all ok |
| `cd v4 && cargo update -p h2 --precise 0.4.16` | h2 0.4.15 → 0.4.16 |
| `cd v4 && cargo audit` | exit 0, 0 vulns, 12 warnings unmaintained/unsound/yanked |
| `cd v4 && cargo test -p cli --offline --test cli_tests test_cli_help` | ok after h2 bump |
| `cd v4/apps/desktop && bun run test -- ProcessTable + Plugins` | 65 passed |
| `cd omnimon_landing && bun test src/lib` | 19 passed |
| `cd omnimon_landing && bun run build` | 16 pages including `/privacy` `/terms` `/es/privacy` `/es/terms` |

Commits:
- macmon `470f6e484e454f9f05ed2fbde1080ef17a47039e` feat(plugins): restrict Lua stdlib and drop sandbox claims
- landing `ed56bcc225bc91f6a18e18cb72179b86aa66e6dc` feat: add privacy/terms pages and cookie-banner links (repo aparte)

## Residual

- Tag `v6.7.0` y DMG Homebrew no publicados.
- `cargo audit` warnings GTK3/unmaintained siguen (CI no falla en warnings).
- `cargo test --workspace` no corrido en local (Tauri + capture; CI lo tiene).
- No Polar, no Sentry, no capdesis-ui, no deploy.
