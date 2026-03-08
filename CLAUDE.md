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
