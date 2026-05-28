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

- Linux CI/CD runs on `ci-runner-node` (`vmi3166182`, public
  `185.237.252.45`, Tailscale `100.120.6.51`) using explicit GitHub Actions
  labels such as `[self-hosted, ci-runner-node, test-light]`,
  `[self-hosted, ci-runner-node, build-heavy]`, and
  `[self-hosted, ci-runner-node, deploy-only]`.
- Staging deploys target `staging-node` (`vmi2875906`, public
  `144.126.159.214`, Tailscale `100.97.107.71`) with staging-only secrets,
  staging domains/routes, and health/smoke/load validation.
- Production stays on `web-app-proxy` / `ancare` (`100.77.243.93`) with the
  shared Traefik edge, production Docker stacks, runtime volumes, and customer
  traffic.
- Databases stay private on `db-architecture` (`100.88.85.128`). Backups and
  storage validation live on `storage-backups` (`100.120.133.78`) and
  `capdesis-nas` (`100.124.183.32`).
- Production promotion happens manually on Monday morning in
  `America/Mexico_City` from the last known-good staging SHA. If Monday is a
  holiday, staging is red, alerts are open, or no operator is available, skip
  the promotion rather than promoting an unverified build.
- `main` may deploy automatically to staging only after CI passes. Production
  must be blocked by failed CI, failed staging deploy, failed health checks,
  failed smoke/k6 thresholds, missing backups for data-changing releases,
  unresolved P0/P1 alerts, or unavailable monitoring.
- Production and staging alerts should converge in `monitor.capdesis.com` /
  Alertmanager/Grafana. Open P0/P1 alerts block Monday promotion.
- Do not move Traefik, production databases, production secrets, or runtime
  production volumes onto `ci-runner-node` or `staging-node` without a
  separate migration plan and validation evidence.

Before changing deploy behavior in this repo, verify the current workflow
labels, staging target, production target, secrets scope, and rollback path
against the canonical docs above.
<!-- CAPDESIS INFRA END -->
