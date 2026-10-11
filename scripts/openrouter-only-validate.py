#!/usr/bin/env python3
"""Fail when product code calls OpenAI directly instead of OpenRouter.

Fleet rule (doc 83, operator decision 2026-10-11): no fleet product calls
api.openai.com and no product runtime reads OPENAI_API_KEY. Every product LLM
call goes through OpenRouter (https://openrouter.ai/api/v1), server-side.

Usage:
  openrouter-only-validate.py --root DIR --app NAME [--paths P ...]
      [--extra-exclude SUB ...]

Scans git-tracked files under --root (git grep, so untracked build output is
never scanned) for the literals `api.openai.com` and `OPENAI_API_KEY`
(case-insensitive). Documentation, vendored deps, lockfiles, tests and agent
tooling are excluded by EXCLUDED_SEGMENTS / EXCLUDED_SUFFIXES /
EXCLUDED_BASENAMES below. --extra-exclude adds path-substring excludes for
documented exceptions (one substring per flag repetition).

Exit 0 when at least one file was scanned and no violation survived the
exclusions. Exit 1 on violation or when zero files were scanned (a scan that
sees nothing is a false green, so it fails closed). Exit 2 on usage errors
or when --root is not a git checkout. No network calls.

Vendored copy: the source of truth is
capdesis-ci-nodes/scripts/openrouter-only-validate.py (this repo is public
and cannot resolve that private reusable workflow, so the check runs from
this local copy in .github/workflows/omnimon-ci.yml). Re-sync by copying
the file again when the source changes.
"""

import argparse
import subprocess
import sys

PATTERN = r"api\.openai\.com|OPENAI_API_KEY"

# Path segments (any depth) that are never production code.
EXCLUDED_SEGMENTS = frozenset(
    [
        ".agents",
        ".claude",
        ".codex",
        ".cursor",
        ".dart_tool",
        ".git",
        ".opencode",
        ".venv",
        "__tests__",
        "audits",
        "build",
        "coverage",
        "dist",
        "docs",
        "e2e-fixtures",
        "fixtures",
        "node_modules",
        "playwright-report",
        "pods",
        "specs",
        "target",
        "test",
        "test-results",
        "testing",
        "tests",
        "tmp",
        "venv",
        "vendor",
        "worktrees",
    ]
)

# File suffixes that are never production code.
EXCLUDED_SUFFIXES = (
    ".lock",
    ".log",
    ".markdown",
    ".md",
    ".orig",
    ".rej",
    ".rst",
)

# Basenames that are never production code (lockfiles, sums, manifests).
EXCLUDED_BASENAMES = frozenset(
    [
        ".gitleaks.toml",
        "CHANGELOG",
        "CHANGELOG.md",
        "go.sum",
    ]
)

# Guard-owned files: the validator, the reusable workflow that embeds it,
# and the caller shims all name the forbidden literals in their own
# comments/pattern, so the guard never scans itself. These files are tiny
# and reviewed on their own PRs; production code cannot hide behind these
# names without a reviewer seeing it.
SELF_EXCLUDED_BASENAMES = frozenset(
    [
        "openrouter-only-validate.py",
        "openrouter-only-gate.yml",
        "openrouter-only.yml",
    ]
)


def excluded(path, extra):
    lowered = path.lower()
    for sub in extra:
        if sub and sub.lower() in lowered:
            return True
    parts = lowered.replace("\\", "/").split("/")
    for part in parts:
        if part in EXCLUDED_SEGMENTS:
            return True
    base = parts[-1] if parts else ""
    if base in EXCLUDED_BASENAMES:
        return True
    if base in SELF_EXCLUDED_BASENAMES:
        return True
    for suffix in EXCLUDED_SUFFIXES:
        if base.endswith(suffix):
            return True
    # Test files next to sources: *_test.go, foo.test.ts, test_foo.py, *_test.dart
    stem = base.rsplit(".", 1)[0] if "." in base else base
    if stem.endswith("_test") or stem.startswith("test_") or ".test." in base:
        return True
    return False


def run_git(root, args):
    proc = subprocess.run(
        ["git", "-c", "core.fsmonitor=false"] + args,
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return proc


def main(argv):
    parser = argparse.ArgumentParser(description="Reject direct-OpenAI usage in product code.")
    parser.add_argument("--root", required=True, help="Repo checkout to scan.")
    parser.add_argument("--app", required=True, help="App name, for messages only.")
    parser.add_argument("--paths", nargs="*", default=["."], help="Git pathspecs to include.")
    parser.add_argument(
        "--extra-exclude",
        action="append",
        default=[],
        help="Extra path-substring exclude (repeatable).",
    )
    args = parser.parse_args(argv)

    root = args.root
    probe = run_git(root, ["rev-parse", "--show-toplevel"])
    if probe.returncode != 0:
        print(f"::error::app={args.app} root is not a git checkout: {root}")
        return 2

    listed = run_git(root, ["ls-files", "-z", "--"] + args.paths)
    if listed.returncode != 0:
        print(f"::error::app={args.app} git ls-files failed: {listed.stderr.strip()}")
        return 2
    tracked = [p for p in listed.stdout.split("\0") if p]
    scannable = [p for p in tracked if not excluded(p, args.extra_exclude)]
    if not scannable:
        print(
            f"::error::app={args.app} openrouter-only scan covered 0 files "
            f"({len(tracked)} tracked, all excluded); refusing a vacuous pass"
        )
        return 1

    found = run_git(
        root, ["grep", "-n", "-i", "-I", "-E", PATTERN, "--"] + args.paths
    )
    if found.returncode not in (0, 1):
        print(f"::error::app={args.app} git grep failed: {found.stderr.strip()}")
        return 2
    violations = []
    for line in found.stdout.splitlines():
        path = line.split(":", 1)[0]
        if not excluded(path, args.extra_exclude):
            violations.append(line)
    if violations:
        print(
            f"::error::app={args.app} direct OpenAI usage in production code "
            f"({len(violations)} hit(s)); route via OpenRouter (doc 83)"
        )
        for line in violations[:50]:
            print(f"::error::{line}")
        if len(violations) > 50:
            print(f"::error::... and {len(violations) - 50} more hit(s)")
        return 1

    print(
        f"PASS app={args.app} openrouter-only: "
        f"{len(scannable)} file(s) scanned, 0 direct-OpenAI references"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
