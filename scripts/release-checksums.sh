#!/usr/bin/env bash
# release-checksums.sh - Writes SHA256SUMS.txt for the artifacts in a directory.
#
# Run it only after the release assets have their final, published names
# (rename-assets). A checksum file written earlier names files that no longer
# exist, and `sha256sum -c` then fails on every line.
#
# Usage: scripts/release-checksums.sh <dir>
set -euo pipefail

DIR="${1:?usage: $0 <dir>}"
[ -d "$DIR" ] || { echo "ERROR: not a directory: $DIR" >&2; exit 1; }

cd "$DIR"
rm -f SHA256SUMS.txt
count=0
for f in *; do
  [ -f "$f" ] || continue
  case "$f" in
    SHA256SUMS*|*.sig.json|releases.json) continue ;;
  esac
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" >> SHA256SUMS.txt
  else
    shasum -a 256 "$f" >> SHA256SUMS.txt
  fi
  count=$((count + 1))
done

if [ "$count" -eq 0 ]; then
  echo "ERROR: no artifacts to checksum in $DIR" >&2
  exit 1
fi
echo "SHA256SUMS.txt written for $count artifacts:"
cat SHA256SUMS.txt
