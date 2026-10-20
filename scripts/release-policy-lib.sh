#!/usr/bin/env bash
# Helpers shared by .github/workflows/release-policy.yml and its regression test.
# Sourced, not executed.

has_top_level_key() {
  local file="$1"
  local key="$2"
  grep -Eq "^${key}:" "$file"
}

# Matches the pattern against workflow content with comment-only lines removed,
# so a "# ... production ..." comment is not treated as a real reference.
# The body is captured in a variable (not piped into grep -q) so that
# `set -o pipefail` cannot turn an early-exit SIGPIPE into a false negative.
workflow_has() {
  local file="$1"
  local pattern="$2"
  local body
  body="$(grep -Ev '^[[:space:]]*#' "$file" || true)"
  grep -Eiq -- "$pattern" <<<"$body"
}
