#!/usr/bin/env bash
# release-assets.sh - GitHub release asset helpers for the omnimon release lane.
#
# The release lane creates a DRAFT release, so tag lookups such as
# `gh release download <tag>` or `releases/tags/<tag>` do not see it. Every
# command here resolves the release by listing releases and matching tag_name.
#
# Usage:
#   release-assets.sh release-id <tag>              print the id of the single release for <tag>
#   release-assets.sh list <tag>                    print "id|name" per asset, newest first
#   release-assets.sh download <tag> <dir>          download every asset of the release into <dir>
#   release-assets.sh upload <tag> <file>           upload <file>, replacing an asset with the same name
#   release-assets.sh check-names                   read asset names from stdin, fail if the set is incomplete
#   release-assets.sh publish <tag>                 publish the draft, only if its asset set is complete
#
# Requires gh authenticated through GH_TOKEN. REPO defaults to GITHUB_REPOSITORY.
set -euo pipefail

REPO="${REPO:-${GITHUB_REPOSITORY:-}}"

die() {
  echo "::error::$*" >&2
  exit 1
}

release_id() {
  local tag="$1" ids count
  [ -n "$REPO" ] || die "REPO or GITHUB_REPOSITORY must be set"
  ids="$(gh api "repos/${REPO}/releases" --paginate --jq ".[] | select(.tag_name == \"${tag}\") | .id")"
  count="$(printf '%s\n' "$ids" | sed '/^$/d' | wc -l | tr -d ' ')"
  if [ "$count" -ne 1 ]; then
    die "expected exactly one release for ${tag}, found ${count}"
  fi
  printf '%s\n' "$ids" | sed '/^$/d'
}

list_assets() {
  local id="$1"
  gh api "repos/${REPO}/releases/${id}" --jq '.assets | sort_by(.created_at) | reverse | .[] | "\(.id)|\(.name)"'
}

# Meta files that accompany the binaries. Everything else is a binary.
is_meta() {
  case "$1" in
    SHA256SUMS*|*.sig.json|releases.json) return 0 ;;
    *) return 1 ;;
  esac
}

check_names() {
  local names missing=0 name ext
  names="$(cat)"
  for ext in dmg AppImage deb rpm msi; do
    if ! printf '%s\n' "$names" | grep -q "\.${ext}\$"; then
      echo "::error::release is missing a .${ext} asset" >&2
      missing=1
    fi
  done
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    is_meta "$name" && continue
    if ! printf '%s\n' "$names" | grep -qxF "${name}.sig.json"; then
      echo "::error::binary ${name} has no .sig.json" >&2
      missing=1
    fi
  done <<< "$names"
  for name in SHA256SUMS.txt releases.json; do
    if ! printf '%s\n' "$names" | grep -qxF "$name"; then
      echo "::error::release is missing ${name}" >&2
      missing=1
    fi
  done
  [ "$missing" -eq 0 ] || exit 1
  echo "Release asset set is complete."
}

cmd="${1:-}"
case "$cmd" in
  release-id)
    [ $# -eq 2 ] || die "usage: $0 release-id <tag>"
    release_id "$2"
    ;;

  list)
    [ $# -eq 2 ] || die "usage: $0 list <tag>"
    list_assets "$(release_id "$2")"
    ;;

  download)
    [ $# -eq 3 ] || die "usage: $0 download <tag> <dir>"
    id="$(release_id "$2")"
    mkdir -p "$3"
    list_assets "$id" | while IFS='|' read -r asset_id name; do
      [ -n "$name" ] || continue
      gh api -H 'Accept: application/octet-stream' "repos/${REPO}/releases/assets/${asset_id}" > "${3}/${name}"
    done
    ls -l "$3"
    ;;

  upload)
    [ $# -eq 3 ] || die "usage: $0 upload <tag> <file>"
    [ -f "$3" ] || die "not a file: $3"
    id="$(release_id "$2")"
    name="$(basename "$3")"
    list_assets "$id" | while IFS='|' read -r asset_id existing; do
      if [ "$existing" = "$name" ]; then
        gh api -X DELETE "repos/${REPO}/releases/assets/${asset_id}" >/dev/null
      fi
    done
    gh api --method POST \
      -H "Content-Type: application/octet-stream" \
      "https://uploads.github.com/repos/${REPO}/releases/${id}/assets?name=${name}" \
      --input "$3" >/dev/null
    echo "Uploaded ${name} to ${2}"
    ;;

  check-names)
    check_names
    ;;

  publish)
    [ $# -eq 2 ] || die "usage: $0 publish <tag>"
    id="$(release_id "$2")"
    names="$(list_assets "$id" | cut -d'|' -f2)"
    printf '%s\n' "$names" | check_names >/dev/null || die "refusing to publish ${2}: asset set incomplete"
    gh api -X PATCH "repos/${REPO}/releases/${id}" -F draft=false >/dev/null
    echo "Published ${2} (release ${id})."
    ;;

  *)
    die "usage: $0 {release-id|list|download|upload|check-names|publish} ..."
    ;;
esac
