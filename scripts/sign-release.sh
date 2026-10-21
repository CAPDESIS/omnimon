#!/usr/bin/env bash
# sign-release.sh - Ed25519 release signing compatible with `omnimon release verify`.
#
# Usage:
#   ED25519_SIGNING_KEY=<key> scripts/sign-release.sh <artifact> <version>
#   ED25519_SIGNING_KEY=<key> scripts/sign-release.sh --manifest <releases.json>
#   ED25519_SIGNING_KEY=<key> scripts/sign-release.sh --check-key
#
# ED25519_SIGNING_KEY is the base64 32-byte Ed25519 seed written by
# `omnimon release generate-keypair` (crypto::export_signing_key).
#
# Fail-closed rules:
#   - A missing key is an error. There is no "sign later" path and no warning.
#   - The key must derive to the public key committed in
#     v4/crates/core/src/release-signing.pub (the key this build trusts).
#     A key that does not match that pin is refused.
#   - Every signature is verified with the derived public key before it is
#     written. The output .sig.json matches crypto::ReleaseSignature.
#
# RELEASE_PINNED_PUBKEY_FILE overrides the pinned-key path (used by tests).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PINNED_FILE="${RELEASE_PINNED_PUBKEY_FILE:-${ROOT_DIR}/v4/crates/core/src/release-signing.pub}"

die() {
  echo "ERROR: $*" >&2
  exit 1
}

usage() {
  echo "Usage: $0 <artifact> <version> | --manifest <releases.json> | --check-key" >&2
  exit 2
}

TMP_DIR="$(mktemp -d)"
chmod 700 "$TMP_DIR"
trap 'rm -rf "$TMP_DIR"' EXIT

command -v openssl >/dev/null 2>&1 || die "openssl is required for Ed25519 signing"
command -v python3 >/dev/null 2>&1 || die "python3 is required to write signature JSON"

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

# Loads the signing key, checks it against the pin, and leaves the PKCS#8 key
# in $TMP_DIR/key.pem and the derived public key in $TMP_DIR/pub.pem.
load_signing_key() {
  local pinned derived
  pinned="$(tr -d '[:space:]' < "$PINNED_FILE" 2>/dev/null || true)"
  [ -n "$pinned" ] || die "no release public key is pinned in ${PINNED_FILE}; commit the public half of the signing key first"

  [ -n "${ED25519_SIGNING_KEY:-}" ] || die "ED25519_SIGNING_KEY is not set: refusing to produce unsigned release artifacts"

  printf '%s' "$ED25519_SIGNING_KEY" | tr -d '[:space:]' > "$TMP_DIR/seed.b64"
  openssl base64 -d -A -in "$TMP_DIR/seed.b64" -out "$TMP_DIR/seed.bin" 2>/dev/null \
    || die "ED25519_SIGNING_KEY is not valid base64"
  [ "$(wc -c < "$TMP_DIR/seed.bin" | tr -d ' ')" -eq 32 ] \
    || die "ED25519_SIGNING_KEY must decode to a 32-byte Ed25519 seed"

  # PKCS#8 wrapper for an Ed25519 private key: 16-byte prefix + 32-byte seed.
  printf '\x30\x2e\x02\x01\x00\x30\x05\x06\x03\x2b\x65\x70\x04\x22\x04\x20' > "$TMP_DIR/key.der"
  cat "$TMP_DIR/seed.bin" >> "$TMP_DIR/key.der"
  openssl pkey -inform DER -in "$TMP_DIR/key.der" -out "$TMP_DIR/key.pem" 2>/dev/null \
    || die "could not load the Ed25519 signing key"

  openssl pkey -in "$TMP_DIR/key.pem" -pubout -out "$TMP_DIR/pub.pem" 2>/dev/null \
    || die "could not derive the public key"
  derived="$(openssl pkey -in "$TMP_DIR/key.pem" -pubout -outform DER 2>/dev/null | tail -c 32 | openssl base64 -A)"

  if [ "$derived" != "$pinned" ]; then
    die "ED25519_SIGNING_KEY does not match the pinned release public key (${PINNED_FILE})"
  fi
}

case "${1:-}" in
  --check-key)
    [ $# -eq 1 ] || usage
    load_signing_key
    echo "Release signing key matches the pinned public key."
    exit 0
    ;;

  --manifest)
    [ $# -eq 2 ] || usage
    MANIFEST="$2"
    [ -f "$MANIFEST" ] || die "manifest not found: $MANIFEST"
    load_signing_key
    # Same payload as crypto::manifest_signing_payload in v4/crates/core/src/crypto.rs.
    python3 - "$MANIFEST" "$TMP_DIR/payload.bin" <<'PY'
import json, sys
manifest_path, payload_path = sys.argv[1], sys.argv[2]
with open(manifest_path) as f:
    manifest = json.load(f)
payload = f"omnimon-manifest:{manifest['version']}:{manifest['date']}"
for a in manifest["artifacts"]:
    payload += f"\n{a['filename']}:{a['sha256']}:{a['signature_b64']}:{a['size_bytes']}"
with open(payload_path, "wb") as f:
    f.write(payload.encode("utf-8"))
PY
    openssl pkeyutl -sign -inkey "$TMP_DIR/key.pem" -rawin -in "$TMP_DIR/payload.bin" -out "$TMP_DIR/manifest.sig"
    openssl pkeyutl -verify -pubin -inkey "$TMP_DIR/pub.pem" -rawin -in "$TMP_DIR/payload.bin" -sigfile "$TMP_DIR/manifest.sig" >/dev/null \
      || die "manifest signature failed self-verification"
    python3 - "$MANIFEST" "$TMP_DIR/manifest.sig" <<'PY'
import base64, json, sys
manifest_path, sig_path = sys.argv[1], sys.argv[2]
with open(sig_path, "rb") as f:
    sig = base64.b64encode(f.read()).decode("ascii")
with open(manifest_path) as f:
    manifest = json.load(f)
manifest["manifest_signature"] = sig
with open(manifest_path, "w") as f:
    json.dump(manifest, f, indent=2)
PY
    echo "Manifest signed: $MANIFEST"
    exit 0
    ;;

  -*|"")
    usage
    ;;
esac

[ $# -eq 2 ] || usage
ARTIFACT="$1"
VERSION="$2"
[ -f "$ARTIFACT" ] || die "artifact not found: $ARTIFACT"

load_signing_key

openssl pkeyutl -sign -inkey "$TMP_DIR/key.pem" -rawin -in "$ARTIFACT" -out "$TMP_DIR/artifact.sig"
openssl pkeyutl -verify -pubin -inkey "$TMP_DIR/pub.pem" -rawin -in "$ARTIFACT" -sigfile "$TMP_DIR/artifact.sig" >/dev/null \
  || die "signature of $ARTIFACT failed self-verification"

SHA256="$(sha256_of "$ARTIFACT")"
SIGNATURE_B64="$(openssl base64 -A -in "$TMP_DIR/artifact.sig")"
PUBLIC_B64="$(openssl pkey -pubin -in "$TMP_DIR/pub.pem" -outform DER | tail -c 32 | openssl base64 -A)"

python3 - "$ARTIFACT" "$VERSION" "$SHA256" "$SIGNATURE_B64" "$PUBLIC_B64" <<'PY'
import json, os, sys
artifact, version, sha, sig, pub = sys.argv[1:6]
out = {
    "version": version,
    "sha256": sha,
    "signature_b64": sig,
    "public_key_b64": pub,
    "artifact": os.path.basename(artifact),
}
with open(artifact + ".sig.json", "w") as f:
    json.dump(out, f, indent=2)
    f.write("\n")
PY

echo "Signature manifest written to ${ARTIFACT}.sig.json"
