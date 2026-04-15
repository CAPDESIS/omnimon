# OmniMon macmon Agent Guide

This repository contains the main OmniMon workspace. The real source of truth
for most product code lives under `v4/`.

## Repo layout

- `v4/apps/desktop/`: Svelte + TypeScript + Vite desktop frontend
- `v4/apps/desktop/src-tauri/`: Tauri host app
- `v4/crates/core/`: shared Rust logic
- `v4/crates/cli/`: CLI crate
- `v4/crates/tui/`: terminal UI crate
- `omnimon_landing/`: landing-site area; treat generated or cache-only content
  carefully

## Commands

### Frontend (`v4/apps/desktop`)

- `bun install`
- `bun run build`
- `bun run test`
- `bun run test:e2e`
- `bun run tauri build -- --debug --no-bundle`

### Rust workspace (`v4`)

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`

## Rules

- Use `bun`, not npm or yarn, for the desktop frontend.
- Keep user-facing replies in Spanish, but keep code and comments aligned with
  the repo's existing language conventions.
- Do not rename legacy paths like `macmon/` unless explicitly requested.
- If the landing-site source is incomplete locally, document that clearly and do
  not invent missing commands.
