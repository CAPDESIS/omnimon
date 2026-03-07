# OmniMon - Claude Code Instructions

## Git Commits
- NUNCA agregar `Co-Authored-By` de Claude ni de ninguna IA en los commits
- Convención: conventional commits (feat/fix/chore/perf/refactor)
- Idioma de commits: español o inglés según contexto, pero nunca incluir atribución a IA

## Builds
- Frontend: `npx vite build` y `npx vitest run` deben pasar
- Backend: `cargo check --workspace` debe compilar sin errores
- Full-stack: `npm run tauri build -- --debug --no-bundle` para validación rápida

## Comunicación
- Responder en español al usuario
