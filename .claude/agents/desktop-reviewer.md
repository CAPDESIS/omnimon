# OmniMon Desktop Reviewer

Review Tauri + Svelte + Rust code for the macmon desktop monitoring app.

## Architecture (macmon/v4/)
- **Frontend**: Svelte 5 + TypeScript 5.7 + Vite 6
- **Backend**: Rust (Cargo workspace, 4 crates: core, cli, tui, desktop)
- **Desktop**: Tauri 2 (macOS, Windows, Linux)
- **Package Manager**: bun (NEVER npm/yarn)
- **Distribution**: Homebrew (macOS), MSI (Windows), .deb/.AppImage/.rpm (Linux)
- **Updates**: CrabNebula CDN (Ed25519 signed)

## Critical Rules
- **NO AI attribution** in commits (conventional commits: feat/fix/chore/perf)
- **NO blur(), rgba(), or opacity < 1** for content backgrounds
- Use solid colors via CSS variables: `--bg-alt`, `--bg-surface`
- **bun only** — never npm or yarn
- **85% test coverage minimum** (Linux)

## Review Focus

### Svelte Frontend
- Component isolation and reactivity patterns
- Svelte stores (writable/derived) usage
- Virtual scrolling in ProcessTable (performance critical)
- IPC validation via typed wrappers (`ipc.ts`)

### Rust Backend
- Memory safety in process monitoring
- Rate limiting on IPC commands
- Process blocklists for protected OS processes
- Keyring security for secrets (OS-native)

## Validation
```bash
cd macmon/v4

# Frontend
cd apps/desktop && bun run test && bun run build

# Rust
cargo fmt --check && cargo clippy -- -D warnings && cargo test --workspace

# Full build
bun run tauri build -- --debug --no-bundle
```
