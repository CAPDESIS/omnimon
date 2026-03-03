#!/usr/bin/env bash
set -euo pipefail

allowed_name="chochy2001"
allowed_email="54371626+chochy2001@users.noreply.github.com"

range="${1:-HEAD}"
invalid=0

while IFS='|' read -r sha author_name author_email; do
    [[ -n "$sha" ]] || continue
    if [[ "$author_name" != "$allowed_name" || "$author_email" != "$allowed_email" ]]; then
        echo "ERROR: unauthorized commit author in $sha"
        echo "  Found:  $author_name <$author_email>"
        echo "  Expected: $allowed_name <$allowed_email>"
        invalid=1
    fi
done < <(git log --format='%H|%an|%ae' "$range")

if (( invalid != 0 )); then
    exit 1
fi

echo "Author verification passed for range: $range"
