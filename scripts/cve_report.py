#!/usr/bin/env python3
"""Audit both dependency graphs; missing tools and network errors are not clean."""

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
import sys
import time


def bun_findings(document):
    if not isinstance(document, dict):
        raise ValueError("Bun advisory report must be an object")
    findings = []
    for package, advisories in document.items():
        if not isinstance(advisories, list):
            raise ValueError("Bun returned an error or malformed advisory report")
        for advisory in advisories:
            if not isinstance(advisory, dict) or advisory.get("severity") not in (
                "low", "moderate", "high", "critical",
            ):
                raise ValueError("Bun returned an invalid advisory")
            if advisory["severity"] in ("high", "critical"):
                findings.append({"package": package, **advisory})
    return findings


def rust_findings(document):
    if not isinstance(document, dict):
        raise ValueError("Rust advisory report must be an object")
    vulnerabilities = document.get("vulnerabilities", {})
    if not isinstance(vulnerabilities, dict) or not isinstance(vulnerabilities.get("list"), list):
        raise ValueError("Rust advisory report lacks vulnerabilities.list")
    findings = vulnerabilities["list"]
    if any(not isinstance(entry, dict) or not entry.get("advisory", {}).get("id") for entry in findings):
        raise ValueError("Rust returned an invalid advisory")
    if "count" in vulnerabilities and vulnerabilities["count"] != len(findings):
        raise ValueError("Rust advisory count differs from list length")
    return findings


def audit(command, cwd, parse_findings, attempts):
    error = "audit did not run"
    for attempt in range(1, attempts + 1):
        try:
            result = subprocess.run(command, cwd=cwd, capture_output=True, text=True, timeout=120)
        except FileNotFoundError:
            return {"status": "infra", "error": command[0] + " is not installed", "attempts": attempt}
        except (OSError, subprocess.TimeoutExpired):
            error = command[0] + " failed or timed out"
        else:
            try:
                document = json.loads(result.stdout)
                findings = parse_findings(document)
                expected_exit = 1 if findings else 0
                if result.returncode != expected_exit:
                    raise ValueError("audit exit code does not agree with its report")
                return {
                    "status": "findings" if findings else "clean",
                    "findings": findings, "report": document, "attempts": attempt,
                }
            except (ValueError, TypeError, AttributeError):
                error = command[0] + " returned an invalid report or failed (exit %s)" % result.returncode
        if attempt < attempts:
            time.sleep(1)
    return {"status": "infra", "error": error, "attempts": attempts}


def frontend_lock_errors(root):
    frontend = root / "v4/apps/desktop"
    errors = []
    for required in ("package.json", "bun.lock"):
        if not (frontend / required).is_file():
            errors.append("missing frontend " + required)
    for obsolete in ("package-lock.json", "npm-shrinkwrap.json", "yarn.lock", "pnpm-lock.yaml"):
        if (frontend / obsolete).exists():
            errors.append("unexpected " + obsolete + "; desktop uses bun.lock")
    return errors


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="print one consolidated JSON object")
    parser.add_argument("--project-root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--report-dir", type=Path, help="default: <project-root>/reports")
    parser.add_argument("--attempts", type=int, choices=range(1, 4), default=3)
    args = parser.parse_args(argv)
    root = args.project_root.resolve()
    errors = frontend_lock_errors(root)
    if errors:
        bun = {"status": "infra", "error": "; ".join(errors), "attempts": 0}
    else:
        bun = audit(["bun", "audit", "--json", "--audit-level=high"], root / "v4/apps/desktop", bun_findings, args.attempts)
    rust_lock = root / "v4/Cargo.lock"
    if (root / "v4/crates/core/Cargo.lock").exists():
        rust = {"status": "infra", "error": "unexpected v4/crates/core/Cargo.lock; core uses workspace Cargo.lock", "attempts": 0}
    elif rust_lock.is_file():
        rust = audit(["cargo", "audit", "--json", "--file", str(rust_lock)], root / "v4", rust_findings, args.attempts)
    else:
        rust = {"status": "infra", "error": "missing v4/Cargo.lock", "attempts": 0}
    statuses = {bun["status"], rust["status"]}
    status = "infra" if "infra" in statuses else "findings" if "findings" in statuses else "clean"
    report = {
        "timestamp": datetime.now(timezone.utc).isoformat(), "status": status,
        "policy": "all Rust advisories; HIGH/CRITICAL Bun advisories; no dependency groups omitted",
        "bun_audit": bun, "rust_audit": rust,
    }
    report_dir = args.report_dir or root / "reports"
    try:
        report_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
        stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
        destination = report_dir / ("cve-report-%s.json" % stamp)
        with destination.open("x") as output:
            destination.chmod(0o600)
            json.dump(report, output, indent=2)
            output.write("\n")
    except OSError:
        report["status"] = status = "infra"
        report["report_error"] = "could not save consolidated report"
    if args.json:
        print(json.dumps(report))
    else:
        for name, result in (("Bun", bun), ("Rust", rust)):
            prefix = {"clean": "CLEAN", "findings": "FINDING", "infra": "INFRA"}[result["status"]]
            print("%s %s: %s" % (prefix, name, result.get("error", "%d advisories" % len(result.get("findings", [])))))
        if "report_error" in report:
            print("INFRA " + report["report_error"])
    return {"clean": 0, "findings": 1, "infra": 2}[status]


if __name__ == "__main__":
    sys.exit(main())
