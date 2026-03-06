#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

STRICT_FLAG="${1:-}"
if [[ "${STRICT_FLAG}" == "--strict-e2e" ]]; then
  export E2E_STRICT=1
fi

echo "[1/5] cargo fmt --check"
cargo fmt --all --check --manifest-path "${ROOT_DIR}/v4/Cargo.toml"

echo "[2/5] cargo clippy -D warnings"
cargo clippy --workspace --all-targets --manifest-path "${ROOT_DIR}/v4/Cargo.toml" -- -D warnings

echo "[3/5] cargo test"
cargo test --manifest-path "${ROOT_DIR}/v4/Cargo.toml"

echo "[4/5] npm run test (frontend)"
npm run test --prefix "${ROOT_DIR}/v4/apps/desktop"

echo "[5/5] npm run test:e2e"
npm run test:e2e --prefix "${ROOT_DIR}/v4/apps/desktop"

echo "All checks passed."
