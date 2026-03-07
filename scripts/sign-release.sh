#!/usr/bin/env bash
# sign-release.sh - Signs release artifacts with Ed25519 and generates SHA-256 checksums
# Usage: ./scripts/sign-release.sh <artifact_path> <version>
#
# Environment variables:
#   ED25519_SIGNING_KEY - Base64-encoded Ed25519 private key (injected via GitHub Secrets)
#
# The public key is embedded in the app binary for offline verification.

set -euo pipefail

# --- Argument validation ---
if [ $# -lt 2 ]; then
    echo "Usage: $0 <artifact_path> <version>"
    echo "Environment: ED25519_SIGNING_KEY (base64-encoded private key)"
    exit 1
fi

ARTIFACT="$1"
VERSION="$2"

if [ ! -f "$ARTIFACT" ]; then
    echo "ERROR: Artifact not found: $ARTIFACT"
    exit 1
fi

# --- SHA-256 checksum (cross-platform) ---
if command -v sha256sum &> /dev/null; then
    SHA256=$(sha256sum "$ARTIFACT" | awk '{print $1}')
elif command -v shasum &> /dev/null; then
    SHA256=$(shasum -a 256 "$ARTIFACT" | awk '{print $1}')
else
    echo "ERROR: Neither sha256sum nor shasum found"
    exit 1
fi

echo "SHA-256: $SHA256"
echo "$SHA256  $(basename "$ARTIFACT")" > "${ARTIFACT}.sha256"
echo "Checksum written to ${ARTIFACT}.sha256"

# --- Ed25519 signature via openssl ---
if [ -z "${ED25519_SIGNING_KEY:-}" ]; then
    echo "WARNING: ED25519_SIGNING_KEY not set. Only SHA-256 checksum generated."
    echo "Set this secret in GitHub Actions to enable release signing."
    exit 0
fi

if ! command -v openssl &> /dev/null; then
    echo "ERROR: openssl is required for Ed25519 signing but was not found."
    exit 1
fi

# Decode the base64 private key to a secure temp file
TMPKEY=$(mktemp)
trap 'rm -f "$TMPKEY"' EXIT
chmod 600 "$TMPKEY"

# Cross-platform base64 decode
if base64 --decode /dev/null 2>/dev/null; then
    echo "$ED25519_SIGNING_KEY" | base64 --decode > "$TMPKEY"
else
    echo "$ED25519_SIGNING_KEY" | base64 -d > "$TMPKEY"
fi

# Sign the artifact
SIGNATURE_B64=$(openssl pkeyutl -sign -inkey "$TMPKEY" -rawin -in "$ARTIFACT" 2>/dev/null | base64 -w 0 2>/dev/null || openssl pkeyutl -sign -inkey "$TMPKEY" -rawin -in "$ARTIFACT" 2>/dev/null | base64)

if [ -z "$SIGNATURE_B64" ]; then
    echo "ERROR: Ed25519 signing failed"
    exit 1
fi

# Generate signature manifest
cat > "${ARTIFACT}.sig.json" <<EOF
{
    "version": "$VERSION",
    "sha256": "$SHA256",
    "signature_b64": "$SIGNATURE_B64",
    "artifact": "$(basename "$ARTIFACT")"
}
EOF

echo "Signature manifest written to ${ARTIFACT}.sig.json"
echo "Release signing complete for $VERSION"
