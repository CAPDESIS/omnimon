#!/usr/bin/env bash
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/omnimon-demo-test.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
source_file="$repo/v4/apps/desktop/src-tauri/src/demo_mode.rs"
for assertions in yes no; do
  rustc --test -C "debug-assertions=$assertions" "$source_file" -o "$scratch/gate-test"
  "$scratch/gate-test"
done
