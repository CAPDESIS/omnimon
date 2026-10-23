#!/usr/bin/env python3
"""Regression guard: the desktop Playwright E2E suite must stay wired in CI.

Fails if .github/workflows/omnimon-ci.yml loses the e2e job, the push
trigger, the per-SHA push concurrency group, or gains a continue-on-error
on the e2e main path. Run: python3 scripts/check-e2e-ci.py
"""
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    sys.exit("PyYAML required: python3 -c 'import yaml' failed")

wf = Path(__file__).resolve().parent.parent / ".github" / "workflows" / "omnimon-ci.yml"
doc = yaml.safe_load(wf.read_text())
# YAML 1.1 parses bare `on` as True.
triggers = doc.get(True) or doc.get("on") or {}
errors = []

if "main" not in (triggers.get("push") or {}).get("branches", []):
    errors.append("push trigger on main missing")

group = str((doc.get("concurrency") or {}).get("group", ""))
if "github.event_name == 'push'" not in group or "github.sha" not in group:
    errors.append("concurrency group is not per-SHA on push")

job = (doc.get("jobs") or {}).get("e2e")
if job is None:
    errors.append("jobs.e2e missing")
else:
    if "continue-on-error" in job:
        errors.append("jobs.e2e must not set continue-on-error")
    runs = " ".join(str(s.get("run", "")) for s in job.get("steps", []))
    if "test:e2e" not in runs:
        errors.append("jobs.e2e does not run the Playwright suite (test:e2e)")

if errors:
    print("E2E CI guard FAILED:")
    [print(f" - {e}") for e in errors]
    sys.exit(1)
print("E2E CI guard OK: push=main, concurrency per-SHA on push, jobs.e2e runs test:e2e, no continue-on-error")
