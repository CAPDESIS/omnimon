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

## Shared UI library: capdesis-ui (consume, do not duplicate)

This repo is part of the Capdesis fleet, which has a shared, versioned component
library: **`github.com/CAPDESIS/capdesis-ui`** (private). Before building or
duplicating any UI, check its catalog: `capdesis-ui/COMPONENTS.md`.

- It ships 40+ TDD-validated components for **Astro** (`@capdesis/ui-astro`) and
  **Flutter** (`package:capdesis_ui`), with a React port in progress: buttons,
  text fields (with password reveal), store-download badges, social links,
  status badges, cards, avatars, dialogs, toggles, loading states, layout
  scaffolds, and more.
- **Consume** these instead of re-implementing UI. Pin the release tag (current
  `v0.2.0`) and bump it to pick up newer components fleet-wide.
- If a component you need is **missing**, EXTRACT it into capdesis-ui (so every
  app gets it) rather than hand-rolling a one-off copy here.

This is the fleet modularization model: reusable components plus a versioned
library that apps pin and upgrade.
