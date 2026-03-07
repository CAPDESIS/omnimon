#!/usr/bin/env bash
# sign-release.sh - Signs release artifacts with Ed25519 and generates SHA-256 checksums
# Usage: ./scripts/sign-release.sh <artifact_path> <version> <private_key_b64>
#
# The signing key (ED25519_SIGNING_KEY) should be stored as a GitHub Secret.
# The public key is embedded in the app for verification.

set -euo pipefail

ARTIFACT="$1"
VERSION="$2"
SIGNING_KEY_B64="${3:-$ED25519_SIGNING_KEY}"

if [ ! -f "$ARTIFACT" ]; then
    echo "ERROR: Artifact not found: $ARTIFACT"
    exit 1
fi

# Generate SHA-256
SHA256=$(shasum -a 256 "$ARTIFACT" | awk '{print $1}')
echo "SHA-256: $SHA256"

# Write checksum file
echo "$SHA256  $(basename "$ARTIFACT")" > "${ARTIFACT}.sha256"
echo "Checksum written to ${ARTIFACT}.sha256"

# Sign with Ed25519 using openssl (if available) or delegate to Rust binary
if command -v openssl &> /dev/null; then
    # Decode the base64 private key to a temp file
    TMPKEY=$(mktemp)
    trap 'rm -f "$TMPKEY" "${TMPKEY}.pub"' EXIT
    echo "$SIGNING_KEY_B64" | base64 -d > "$TMPKEY"

    # Sign the artifact
    SIGNATURE_B64=$(openssl pkeyutl -sign -inkey "$TMPKEY" -rawin -in "$ARTIFACT" 2>/dev/null | base64)

    # Generate manifest
    cat > "${ARTIFACT}.sig.json" <<EOF
{
    "version": "$VERSION",
    "sha256": "$SHA256",
    "signature_b64": "$SIGNATURE_B64",
    "artifact": "$(basename "$ARTIFACT")"
}
EOF
    echo "Signature manifest written to ${ARTIFACT}.sig.json"
else
    echo "WARNING: openssl not available. Only SHA-256 checksum generated."
    echo "The CI pipeline will handle Ed25519 signing via the Rust toolchain."
fi

echo "Release signing complete for $VERSION"
