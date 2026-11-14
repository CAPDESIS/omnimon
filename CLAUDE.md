# OmniMon - Claude Code Instructions

## Git Commits
- NUNCA agregar `Co-Authored-By` de Claude ni de ninguna IA en los commits
- Convención: conventional commits (feat/fix/chore/perf/refactor)
- Idioma de commits: español o inglés según contexto, pero nunca incluir atribución a IA

## Builds
- Frontend: `bun run build` y `bun run test` deben pasar
- Backend: `cargo check --workspace` debe compilar sin errores
- Full-stack: `bun run tauri build -- --debug --no-bundle` para validación rápida
- Package manager: **bun** (no npm/node). Lockfile: `bun.lock`

## Comunicación
- Responder en español al usuario

## UI — Prohibido usar transparencias
- **NUNCA** usar `backdrop-filter: blur()` en ningún componente
- **NUNCA** usar `rgba()` en backgrounds (excepto overlay de modales: `rgba(0,0,0,0.7)`)
- **NUNCA** usar `opacity < 1` para fondos o contenido — usar `color: var(--fg-muted)` o `filter: brightness()`
- **NUNCA** usar `color-mix(... transparent)` — usar `color-mix(... var(--bg))` como base sólida
- Fondos siempre **sólidos**: usar variables del tema (`--bg-alt`, `--bg-surface`, `--bg-secondary`)
- Badges/tags de color: `color-mix(in srgb, var(--color) NN%, var(--bg))`
- Disabled state: `filter: grayscale(0.4) brightness(0.7)` en vez de `opacity`
- `box-shadow` con rgba **sí es permitido** (las sombras necesitan transparencia)

## Landing / Releases
- Cada release publico nuevo debe actualizar landing, enlaces de descarga y blog EN/ES
- Los blogs de releases deben mantenerse sincronizados con el changelog real y los assets publicados
- El listado del blog debe quedar siempre ordenado del release mas nuevo al mas viejo

<!-- CAPDESIS INFRA START -->
## CAPDESIS Architecture And Delivery Policy

This repo participates in the shared CAPDESIS workspace architecture. Keep this
block aligned with the canonical local docs under
`/Users/jorge/Documents/Apps/docs/`:

- `TAILSKILL_NEW_VPS_HANDOFF.md`: current VPS, Tailscale, CI runner, staging,
  production, storage, and rollback topology.
- `STAGING_RELEASE_POLICY.md`: CI -> staging -> production release gates.
- `PRODUCTION_ALERTING_RUNBOOK.md`: centralized monitoring and alert routing.
- `APP_RELEASE_READINESS_AUDIT.md`: app deploy workflow state and staging gaps.
- `SCALING_AND_LOAD_TEST_PLAN.md`: staging-first load testing and scaling
  decision rules.
- `INFRASTRUCTURE_COSTS.md`: verified VPS cost baseline and annual estimates.

Current operating model:

- This repository (`CAPDESIS/omnimon`) is **public**, so `.github/workflows/omnimon-ci.yml` runs on GitHub-hosted runners (`ubuntu-latest`, `macos-latest`, `windows-latest`) rather than self-hosted CAPDESIS fleet runners.
- **Staging is not a deploy target.** `staging-node` (`vmi2875906`, Tailscale `100.97.107.71`) is drained with 0 GitHub Actions runners registered (`ci-nodes#230`) and no workflow in this repo references it. Do not describe a staging deploy, a staging SHA, or staging smoke/load validation on `staging-node`.
- Production stays on `web-app-proxy` / `ancare` (`100.77.243.93`) for CAPDESIS web services; OmniMon desktop installers/updater manifests publish via GitHub Releases / CrabNebula CDN (`omnimon-ci.yml`), and `omnimon.com.mx` deploys via `chochy2001/omnimon_landing`.
- Databases stay private on `db-architecture` (`100.88.85.128`). Backups and storage validation live on `storage-backups` (`100.120.133.78`) and `capdesis-nas` (`100.124.183.32`).
- Desktop release promotion uses tagged/dispatched `omnimon-ci.yml` runs after CI passes (no Monday-only window and no `staging-node` staging gate). Failed CI, failed coverage gates, or missing signing keys block release promotion.
- Production fleet alerts converge in `monitor.capdesis.com` / Alertmanager/Grafana. Open P0/P1 fleet alerts block production promotions.
- Do not move Traefik, production databases, production secrets, or runtime production volumes onto `ci-runner-node` or `staging-node` without a separate migration plan and validation evidence.

Before changing deploy behavior in this repo, verify the current workflow
labels, staging target, production target, secrets scope, and rollback path
against the canonical docs above.
<!-- CAPDESIS INFRA END -->

## Shared UI library: capdesis-ui

The Capdesis fleet library (`@capdesis/ui-astro`, `package:capdesis_ui`) is
**not required** for this desktop app. OmniMon desktop UI is Svelte 5 inside
Tauri; do not add `capdesis-ui` or a second Sentry SDK here. Landing
(`omnimon_landing`, separate git repo) stays Astro + Tailwind without
capdesis-ui unless a later product decision says otherwise.

Verified 2026-10-25: zero `capdesis-ui` / `capdesis_ui` / `@capdesis`
references in code or manifests of either repo (desktop `package.json`,
landing `package.json`, Cargo manifests). This section previously said
"consume, do not duplicate", which contradicted `AGENTS.md` and the code;
`AGENTS.md` was correct.
