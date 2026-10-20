#!/usr/bin/env bash
# Regression test: release-policy must not count comment-only lines as
# workflow references (a comment mentioning "production" caused a false WARNING
# on omnimon-ci.yml, run 39452848154 on main a20dd91).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=release-policy-lib.sh
source "${ROOT_DIR}/scripts/release-policy-lib.sh"

fixtures="$(mktemp -d)"
trap 'rm -rf "$fixtures"' EXIT

cat > "$fixtures/comment-only.yml" <<'YAML'
name: CI
# This comment mentions production and environment: production on purpose.
on:
  push:
    branches: [main]
jobs:
  test:
    runs-on: [self-hosted, Linux, X64, test-light]
    steps:
      - run: echo "unit tests"
YAML

cat > "$fixtures/real-environment.yml" <<'YAML'
name: Deploy
on:
  workflow_dispatch:
jobs:
  deploy:
    runs-on: [self-hosted, ci-runner-node, deploy-only]
    environment: production
    steps:
      - run: echo "deploy"
YAML

cat > "$fixtures/real-run-text.yml" <<'YAML'
name: Notify
on:
  workflow_dispatch:
jobs:
  notify:
    runs-on: [self-hosted, Linux, X64, policy-light]
    steps:
      - run: echo "shipping to production now"
YAML

cat > "$fixtures/comment-deploy-tool.yml" <<'YAML'
name: Lint
# rsync and docker compose are only mentioned in this comment.
on:
  push:
    branches: [main]
jobs:
  lint:
    runs-on: [self-hosted, Linux, X64, policy-light]
    steps:
      - run: bun run lint
YAML

PROD_PATTERN="environment:[[:space:]]*production|deploy-prod|production|PRODUCTION_|web-app-proxy|ancare"
DEPLOY_PATTERN="docker compose|rsync|ssh .+@|scp |kubectl|fly deploy|vercel|hostinger"

failures=0
check() {
  local label="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    echo "PASS: ${label} (expected ${expected}, got ${actual})"
  else
    echo "FAIL: ${label} (expected ${expected}, got ${actual})"
    failures=$((failures + 1))
  fi
}

result() {
  if workflow_has "$1" "$2"; then echo true; else echo false; fi
}

check "comment-only production mention is not a production reference" false \
  "$(result "$fixtures/comment-only.yml" "$PROD_PATTERN")"
check "comment-only deploy tool mention is not a deploy reference" false \
  "$(result "$fixtures/comment-deploy-tool.yml" "$DEPLOY_PATTERN")"
check "real environment: production declaration still counts" true \
  "$(result "$fixtures/real-environment.yml" "$PROD_PATTERN")"
check "real run-step text mentioning production still counts" true \
  "$(result "$fixtures/real-run-text.yml" "$PROD_PATTERN")"

if [ "$failures" -gt 0 ]; then
  echo "${failures} check(s) failed"
  exit 1
fi
echo "all release-policy helper checks passed"
