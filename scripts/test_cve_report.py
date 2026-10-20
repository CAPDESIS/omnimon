#!/usr/bin/env python3
"""Exercise the report CLI against actual fake tool processes and files."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


REPORT = Path(__file__).resolve().with_name("cve_report.py")


class CveReportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="omnimon-cve-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.frontend = self.root / "v4/apps/desktop"
        self.frontend.mkdir(parents=True)
        for name in ("package.json", "bun.lock"):
            (self.frontend / name).write_text("{}")
        (self.root / "v4/Cargo.lock").write_text("version = 4\n")
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.env = {
            **os.environ, "PATH": str(self.bin),
            "BUN_JSON": "{}", "BUN_EXIT": "0",
            "RUST_JSON": '{"vulnerabilities":{"list":[],"count":0}}', "RUST_EXIT": "0",
        }
        for name, prefix in (("bun", "BUN"), ("cargo", "RUST")):
            tool = self.bin / name
            tool.write_text(
                "#!%s\nimport os,sys\nprint(os.environ['%s_JSON'])\nsys.exit(int(os.environ['%s_EXIT']))\n"
                % (sys.executable, prefix, prefix)
            )
            tool.chmod(0o700)

    def run_report(self):
        result = subprocess.run(
            [sys.executable, str(REPORT), "--json", "--attempts", "1", "--project-root", str(self.root)],
            env=self.env, capture_output=True, text=True,
        )
        self.assertEqual(result.stderr, "")
        report = json.loads(result.stdout)
        saved = list((self.root / "reports").glob("cve-report-*.json"))
        self.assertEqual(len(saved), 1)
        self.assertEqual(saved[0].stat().st_mode & 0o777, 0o600)
        self.assertEqual(json.loads(saved[0].read_text()), report)
        return result.returncode, report

    def test_clean_graphs_exit_zero(self):
        code, report = self.run_report()
        self.assertEqual((code, report["status"]), (0, "clean"))
        self.assertEqual(report["bun_audit"]["status"], "clean")
        self.assertEqual(report["rust_audit"]["status"], "clean")

    def test_high_bun_advisory_fails(self):
        self.env.update(BUN_JSON=json.dumps({"pkg": [{"id": 123, "severity": "high"}]}), BUN_EXIT="1")
        code, report = self.run_report()
        self.assertEqual((code, report["status"]), (1, "findings"))
        self.assertEqual(report["bun_audit"]["findings"][0]["id"], 123)

    def test_moderate_bun_advisory_is_visible_without_high_alert(self):
        self.env["BUN_JSON"] = json.dumps({"pkg": [{"id": 123, "severity": "moderate"}]})
        code, report = self.run_report()
        self.assertEqual(code, 0)
        self.assertEqual(report["bun_audit"]["report"]["pkg"][0]["severity"], "moderate")

    def test_rust_advisory_fails(self):
        self.env.update(
            RUST_JSON=json.dumps({"vulnerabilities": {"list": [{"advisory": {"id": "RUSTSEC-TEST"}}], "count": 1}}),
            RUST_EXIT="1",
        )
        code, report = self.run_report()
        self.assertEqual((code, report["status"]), (1, "findings"))

    def test_missing_bun_is_infrastructure_failure(self):
        (self.bin / "bun").unlink()
        code, report = self.run_report()
        self.assertEqual((code, report["status"]), (2, "infra"))
        self.assertEqual(report["rust_audit"]["status"], "clean")

    def test_missing_cargo_is_infrastructure_failure(self):
        (self.bin / "cargo").unlink()
        code, report = self.run_report()
        self.assertEqual((code, report["status"]), (2, "infra"))

    def test_non_json_network_failure_is_not_vulnerability_or_clean(self):
        self.env.update(BUN_JSON="<html>proxy failure</html>", BUN_EXIT="1")
        code, report = self.run_report()
        self.assertEqual((code, report["status"]), (2, "infra"))
        self.assertNotIn("findings", report["bun_audit"])

    def test_registry_error_object_is_not_clean(self):
        self.env.update(BUN_JSON='{"error":"unavailable"}', BUN_EXIT="1")
        code, report = self.run_report()
        self.assertEqual((code, report["status"]), (2, "infra"))

    def test_failed_tool_with_empty_report_is_not_clean(self):
        self.env["BUN_EXIT"] = "1"
        code, report = self.run_report()
        self.assertEqual((code, report["status"]), (2, "infra"))

    def test_competing_lockfile_rejected_before_bun_runs(self):
        (self.frontend / "package-lock.json").write_text("{}")
        code, report = self.run_report()
        self.assertEqual(code, 2)
        self.assertIn("unexpected package-lock.json", report["bun_audit"]["error"])
        self.assertEqual(report["bun_audit"]["attempts"], 0)

    def test_missing_lock_is_not_skipped(self):
        (self.frontend / "bun.lock").unlink()
        code, report = self.run_report()
        self.assertEqual((code, report["status"]), (2, "infra"))

    def test_child_cargo_lock_is_rejected_before_rust_audit(self):
        child = self.root / "v4/crates/core/Cargo.lock"
        child.parent.mkdir(parents=True)
        child.write_text("version = 4\n")
        code, report = self.run_report()
        self.assertEqual((code, report["status"]), (2, "infra"))
        self.assertIn("unexpected v4/crates/core/Cargo.lock", report["rust_audit"]["error"])
        self.assertEqual(report["rust_audit"]["attempts"], 0)

    def test_bad_rust_schema_is_infrastructure_failure(self):
        self.env["RUST_JSON"] = "{}"
        code, report = self.run_report()
        self.assertEqual((code, report["status"]), (2, "infra"))


if __name__ == "__main__":
    unittest.main()
