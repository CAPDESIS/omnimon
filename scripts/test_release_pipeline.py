#!/usr/bin/env python3
"""Regression tests for the omnimon release lane.

Two kinds of checks:

* Workflow graph (PyYAML): the release jobs gate each other the way the
  release policy needs. A publish must wait for signing, a missing signing key
  must stop the release before any asset exists, and no publishing step may be
  allowed to fail silently.
* Behaviour (real subprocesses, real openssl): the signing script refuses to
  produce unsigned or wrongly-keyed artifacts, writes the signature format that
  `omnimon release verify` reads, and the checksum file only verifies once it is
  written after the renames.

Run: python3 scripts/test_release_pipeline.py -v
"""

from __future__ import annotations

import base64
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = ROOT / ".github" / "workflows" / "omnimon-ci.yml"
SIGN = ROOT / "scripts" / "sign-release.sh"
CHECKSUMS = ROOT / "scripts" / "release-checksums.sh"
ASSETS = ROOT / "scripts" / "release-assets.sh"

RELEASE_IF = "startsWith(github.ref, 'refs/tags/v')"

try:
    import yaml  # type: ignore
except ImportError:  # pragma: no cover - reported as a test error below
    yaml = None


def openssl(*args: str, input_bytes: bytes | None = None) -> bytes:
    proc = subprocess.run(
        ["openssl", *args],
        input=input_bytes,
        capture_output=True,
        check=True,
    )
    return proc.stdout


def make_keypair(workdir: Path) -> tuple[str, str]:
    """Returns (seed_b64, public_b64) in the format omnimon uses."""
    key_pem = workdir / "key.pem"
    openssl("genpkey", "-algorithm", "ED25519", "-out", str(key_pem))
    der = openssl("pkey", "-in", str(key_pem), "-outform", "DER")
    seed_b64 = base64.b64encode(der[-32:]).decode()
    pub_der = openssl("pkey", "-in", str(key_pem), "-pubout", "-outform", "DER")
    public_b64 = base64.b64encode(pub_der[-32:]).decode()
    return seed_b64, public_b64


def run_sign(args: list[str], env_extra: dict[str, str], cwd: Path) -> subprocess.CompletedProcess:
    env = {k: v for k, v in os.environ.items() if k != "ED25519_SIGNING_KEY"}
    env.update(env_extra)
    return subprocess.run(
        ["bash", str(SIGN), *args],
        cwd=cwd,
        env=env,
        capture_output=True,
        text=True,
    )


def sha256_check(directory: Path) -> subprocess.CompletedProcess:
    if shutil.which("sha256sum"):
        cmd = ["sha256sum", "-c", "SHA256SUMS.txt"]
    else:
        cmd = ["shasum", "-a", "256", "-c", "SHA256SUMS.txt"]
    return subprocess.run(cmd, cwd=directory, capture_output=True, text=True)


@unittest.skipIf(yaml is None, "PyYAML is required for the workflow graph checks")
class WorkflowGraphTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        with WORKFLOW.open() as f:
            cls.wf = yaml.safe_load(f)
        # PyYAML parses the bare key `on` as boolean True.
        cls.triggers = cls.wf.get("on", cls.wf.get(True, {}))
        cls.jobs = cls.wf["jobs"]

    def needs(self, job: str) -> list[str]:
        value = self.jobs[job].get("needs", [])
        return [value] if isinstance(value, str) else list(value)

    def test_push_to_main_runs_validation_and_no_tag_trigger(self) -> None:
        self.assertIn("push", self.triggers)
        self.assertEqual(self.triggers["push"].get("branches"), ["main"])
        self.assertNotIn("tags", self.triggers["push"])

    def test_push_concurrency_is_isolated_per_sha(self) -> None:
        group = self.wf["concurrency"]["group"]
        self.assertIn("github.sha", group)
        self.assertIn("event_name == 'push'", group)

    def test_signing_key_preflight_gates_the_release_build(self) -> None:
        self.assertIn("signing-key-preflight", self.jobs)
        self.assertIn("signing-key-preflight", self.needs("release"))
        self.assertIn(RELEASE_IF, self.jobs["signing-key-preflight"]["if"])
        steps = self.jobs["signing-key-preflight"]["steps"]
        self.assertTrue(any("--check-key" in str(s.get("run", "")) for s in steps))

    def test_publish_waits_for_every_gate(self) -> None:
        needs = set(self.needs("publish-release"))
        self.assertTrue({"release", "rename-assets", "sign-release"} <= needs, needs)

    def test_checksums_and_signatures_run_after_rename(self) -> None:
        self.assertEqual(self.needs("rename-assets"), ["release"])
        self.assertIn("rename-assets", self.needs("sign-release"))
        names = [s.get("name", "") for s in self.jobs["sign-release"]["steps"]]
        checksum_idx = next(i for i, n in enumerate(names) if "checksum" in n.lower())
        sign_idx = next(i for i, n in enumerate(names) if n.lower().startswith("sign every"))
        self.assertLess(checksum_idx, sign_idx)

    def test_release_is_created_as_draft(self) -> None:
        steps = self.jobs["release"]["steps"]
        tauri = [s for s in steps if str(s.get("uses", "")).startswith("tauri-apps/tauri-action")]
        self.assertEqual(len(tauri), 1)
        self.assertIs(tauri[0]["with"]["releaseDraft"], True)

    def test_no_continue_on_error_in_release_path(self) -> None:
        for job_name in ("signing-key-preflight", "release", "rename-assets", "sign-release", "publish-release"):
            job = self.jobs[job_name]
            self.assertNotIn("continue-on-error", job, job_name)
            for step in job.get("steps", []):
                self.assertNotIn("continue-on-error", step, f"{job_name}: {step.get('name')}")

    def test_release_matrix_has_no_experimental_legs(self) -> None:
        matrix = self.jobs["release"]["strategy"]["matrix"]["include"]
        self.assertEqual({leg["platform"] for leg in matrix}, {"linux", "macos-universal", "windows"})
        for leg in matrix:
            self.assertNotIn("experimental", leg, leg)

    def test_no_signing_step_is_skipped_on_missing_secret(self) -> None:
        for job_name in ("release", "sign-release", "publish-release"):
            for step in self.jobs[job_name].get("steps", []):
                condition = str(step.get("if", ""))
                self.assertNotIn("ED25519", condition, f"{job_name}: {step.get('name')}")


class SigningScriptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="omnimon-sign-test-"))
        self.seed, self.public = make_keypair(self.tmp)
        self.pinned = self.tmp / "release-signing.pub"
        self.pinned.write_text(self.public + "\n")
        self.artifact = self.tmp / "OmniMon-6.8.0-Linux.deb"
        self.artifact.write_bytes(b"omnimon release bytes\n")
        self.env = {"RELEASE_PINNED_PUBKEY_FILE": str(self.pinned)}

    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_missing_key_fails_instead_of_skipping(self) -> None:
        result = run_sign([str(self.artifact), "6.8.0"], self.env, self.tmp)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("ED25519_SIGNING_KEY is not set", result.stderr)
        self.assertFalse(Path(str(self.artifact) + ".sig.json").exists())

    def test_empty_pin_fails_closed(self) -> None:
        self.pinned.write_text("")
        result = run_sign(
            [str(self.artifact), "6.8.0"],
            {**self.env, "ED25519_SIGNING_KEY": self.seed},
            self.tmp,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no release public key is pinned", result.stderr)

    def test_key_not_matching_the_pin_is_refused(self) -> None:
        other_seed, _ = make_keypair(Path(tempfile.mkdtemp(dir=self.tmp)))
        result = run_sign(
            [str(self.artifact), "6.8.0"],
            {**self.env, "ED25519_SIGNING_KEY": other_seed},
            self.tmp,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match the pinned release public key", result.stderr)
        self.assertFalse(Path(str(self.artifact) + ".sig.json").exists())

    def test_check_key_accepts_the_pinned_key(self) -> None:
        result = run_sign(["--check-key"], {**self.env, "ED25519_SIGNING_KEY": self.seed}, self.tmp)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_signature_file_matches_verifier_format_and_verifies(self) -> None:
        result = run_sign(
            [str(self.artifact), "6.8.0"],
            {**self.env, "ED25519_SIGNING_KEY": self.seed},
            self.tmp,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        sig = json.loads(Path(str(self.artifact) + ".sig.json").read_text())
        # Fields of crypto::ReleaseSignature in v4/crates/core/src/crypto.rs.
        for field in ("version", "sha256", "signature_b64", "public_key_b64"):
            self.assertIn(field, sig)
        self.assertEqual(sig["public_key_b64"], self.public)
        self.assertEqual(sig["version"], "6.8.0")
        self.assertEqual(sig["artifact"], self.artifact.name)

        # Independent check with openssl against the pinned public key.
        pub_pem = self.tmp / "pinned.pem"
        der = base64.b64decode(self.public)
        # SubjectPublicKeyInfo for Ed25519: 12-byte prefix + 32-byte key.
        spki = bytes.fromhex("302a300506032b6570032100") + der
        spki_file = self.tmp / "pinned.der"
        spki_file.write_bytes(spki)
        openssl("pkey", "-pubin", "-inform", "DER", "-in", str(spki_file), "-out", str(pub_pem))
        sig_bytes = base64.b64decode(sig["signature_b64"])
        sig_file = self.tmp / "check.sig"
        sig_file.write_bytes(sig_bytes)
        verify = subprocess.run(
            ["openssl", "pkeyutl", "-verify", "-pubin", "-inkey", str(pub_pem), "-rawin",
             "-in", str(self.artifact), "-sigfile", str(sig_file)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(verify.returncode, 0, verify.stderr + verify.stdout)

    def test_manifest_is_signed_over_the_verifier_payload(self) -> None:
        sign = run_sign(
            [str(self.artifact), "6.8.0"],
            {**self.env, "ED25519_SIGNING_KEY": self.seed},
            self.tmp,
        )
        self.assertEqual(sign.returncode, 0, sign.stderr)
        sig = json.loads(Path(str(self.artifact) + ".sig.json").read_text())
        manifest = self.tmp / "releases.json"
        manifest.write_text(json.dumps({
            "version": "6.8.0",
            "date": "2026-10-20",
            "artifacts": [{
                "filename": self.artifact.name,
                "platform": "linux",
                "arch": "unknown",
                "sha256": sig["sha256"],
                "signature_b64": sig["signature_b64"],
                "size_bytes": self.artifact.stat().st_size,
            }],
            "manifest_signature": "",
        }))
        result = run_sign(["--manifest", str(manifest)], {**self.env, "ED25519_SIGNING_KEY": self.seed}, self.tmp)
        self.assertEqual(result.returncode, 0, result.stderr)
        signed = json.loads(manifest.read_text())
        self.assertNotEqual(signed["manifest_signature"], "")
        self.assertNotIn("GENERATED_BY_CI", signed["manifest_signature"])

        # Payload layout of crypto::manifest_signing_payload.
        a = signed["artifacts"][0]
        payload = f"omnimon-manifest:{signed['version']}:{signed['date']}\n" \
                  f"{a['filename']}:{a['sha256']}:{a['signature_b64']}:{a['size_bytes']}"
        payload_file = self.tmp / "payload.bin"
        payload_file.write_bytes(payload.encode())
        sig_file = self.tmp / "manifest.sig"
        sig_file.write_bytes(base64.b64decode(signed["manifest_signature"]))
        pub_pem = self.tmp / "pinned.pem"
        spki = bytes.fromhex("302a300506032b6570032100") + base64.b64decode(self.public)
        spki_file = self.tmp / "pinned.der"
        spki_file.write_bytes(spki)
        openssl("pkey", "-pubin", "-inform", "DER", "-in", str(spki_file), "-out", str(pub_pem))
        verify = subprocess.run(
            ["openssl", "pkeyutl", "-verify", "-pubin", "-inkey", str(pub_pem), "-rawin",
             "-in", str(payload_file), "-sigfile", str(sig_file)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(verify.returncode, 0, verify.stderr + verify.stdout)


class ChecksumOrderTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="omnimon-sums-test-"))

    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    def write_assets(self, names: dict[str, bytes]) -> None:
        for name, body in names.items():
            (self.tmp / name).write_bytes(body)

    def test_checksums_written_before_rename_do_not_verify(self) -> None:
        self.write_assets({"OmniMon_6.8.0_amd64.deb": b"deb bytes"})
        subprocess.run(["bash", str(CHECKSUMS), str(self.tmp)], check=True, capture_output=True)
        (self.tmp / "OmniMon_6.8.0_amd64.deb").rename(self.tmp / "OmniMon-6.8.0-Linux.deb")
        self.assertNotEqual(sha256_check(self.tmp).returncode, 0)

    def test_checksums_written_after_rename_verify(self) -> None:
        self.write_assets({"OmniMon_6.8.0_amd64.deb": b"deb bytes"})
        (self.tmp / "OmniMon_6.8.0_amd64.deb").rename(self.tmp / "OmniMon-6.8.0-Linux.deb")
        result = subprocess.run(["bash", str(CHECKSUMS), str(self.tmp)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        check = sha256_check(self.tmp)
        self.assertEqual(check.returncode, 0, check.stdout + check.stderr)
        self.assertIn("OmniMon-6.8.0-Linux.deb: OK", check.stdout)

    def test_checksums_skip_meta_files(self) -> None:
        self.write_assets({
            "OmniMon-6.8.0-Linux.deb": b"deb bytes",
            "OmniMon-6.8.0-Linux.deb.sig.json": b"{}",
            "releases.json": b"{}",
        })
        subprocess.run(["bash", str(CHECKSUMS), str(self.tmp)], check=True, capture_output=True)
        listed = (self.tmp / "SHA256SUMS.txt").read_text()
        self.assertIn("OmniMon-6.8.0-Linux.deb", listed)
        self.assertNotIn("sig.json", listed)
        self.assertNotIn("releases.json", listed)

    def test_checksums_refuse_an_empty_directory(self) -> None:
        result = subprocess.run(["bash", str(CHECKSUMS), str(self.tmp)], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)


class AssetCompletenessTests(unittest.TestCase):
    FULL = [
        "OmniMon-6.8.0-macOS-Universal.dmg", "OmniMon-6.8.0-Windows.msi",
        "OmniMon-6.8.0-Linux.AppImage", "OmniMon-6.8.0-Linux.deb", "OmniMon-6.8.0-Linux.rpm",
    ]

    def names(self, names: list[str]) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["bash", str(ASSETS), "check-names"],
            input="\n".join(names) + "\n",
            capture_output=True,
            text=True,
        )

    def with_sigs(self, binaries: list[str]) -> list[str]:
        out = list(binaries)
        out += [b + ".sig.json" for b in binaries]
        out += ["SHA256SUMS.txt", "releases.json"]
        return out

    def test_complete_set_passes(self) -> None:
        self.assertEqual(self.names(self.with_sigs(self.FULL)).returncode, 0)

    def test_missing_platform_binary_blocks_publication(self) -> None:
        partial = [n for n in self.FULL if not n.endswith(".rpm")]
        result = self.names(self.with_sigs(partial))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(".rpm", result.stderr)

    def test_binary_without_signature_blocks_publication(self) -> None:
        names = self.with_sigs(self.FULL)
        names.remove("OmniMon-6.8.0-Windows.msi.sig.json")
        result = self.names(names)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("OmniMon-6.8.0-Windows.msi has no .sig.json", result.stderr)


class ExampleManifestTests(unittest.TestCase):
    """The only committed manifest is an example, and it says so by its name.

    The real, signed releases.json is generated and attached to each GitHub
    Release by omnimon-ci.yml and is never committed. A committed file named
    releases.json full of PLACEHOLDER_ hashes (v4/releases.json, 6.0.1) looked
    like a real manifest to anyone browsing the tree.
    """

    EXAMPLE = ROOT / "v4" / "releases.example.json"

    def tracked_files(self) -> list[str]:
        listing = subprocess.run(
            ["git", "-C", str(ROOT), "ls-files", "-z"],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(listing.returncode, 0, listing.stderr)
        return [path for path in listing.stdout.split("\0") if path]

    def test_no_manifest_named_releases_json_is_committed(self) -> None:
        tracked = self.tracked_files()
        self.assertGreater(len(tracked), 0)
        offenders = [p for p in tracked if p.split("/")[-1] == "releases.json"]
        self.assertEqual(offenders, [])

    def test_example_manifest_holds_only_placeholders(self) -> None:
        manifest = json.loads(self.EXAMPLE.read_text(encoding="utf-8"))
        self.assertTrue(manifest["manifest_signature"].startswith("PLACEHOLDER_"))
        self.assertGreater(len(manifest["artifacts"]), 0)
        for artifact in manifest["artifacts"]:
            self.assertTrue(artifact["sha256"].startswith("PLACEHOLDER_"), artifact)
            self.assertTrue(artifact["signature_b64"].startswith("PLACEHOLDER_"), artifact)
            self.assertEqual(artifact["size_bytes"], 0, artifact)


if __name__ == "__main__":
    unittest.main()
