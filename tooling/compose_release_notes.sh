#!/usr/bin/env bash
set -euo pipefail

# Compose the body of a GitHub release.
#
# `gh release create --generate-notes` alone produces a bare list of merged pull
# requests, which says nothing about what the release does, which file to
# download, or which image production will pin. This assembles the parts a reader
# actually needs and appends the generated changelog at the end.
#
# usage: compose_release_notes.sh <tag> <repository> <image> <digest> <artifacts-dir> [highlights-file]

tag=${1:-}
repository=${2:-}
image=${3:-}
digest=${4:-}
artifacts=${5:-}
highlights=${6:-}

if [ -z "$tag" ] || [ -z "$repository" ] || [ -z "$image" ] || [ -z "$digest" ] || [ -z "$artifacts" ]; then
  echo "usage: ${0##*/} <tag> <repository> <image> <digest> <artifacts-dir> [highlights-file]" >&2
  exit 1
fi

if ! [[ "$digest" =~ ^sha256:[0-9a-f]{64}$ ]]; then
  echo "Refusing to publish notes without an immutable digest (got '$digest')" >&2
  exit 1
fi

fence='```'

# A machine cannot invent why a release matters. Hand-written highlights are used
# when they exist, and their absence only costs the summary, never the rest.
if [ -n "$highlights" ] && [ -f "$highlights" ]; then
  cat "$highlights"
  printf '\n\n'
fi

printf '## Install\n\n'
printf '| Platform | Artifact |\n'
printf '| --- | --- |\n'

for entry in '*.deb:Linux (Debian/Ubuntu)' '*.AppImage:Linux (portable)' \
  '*-setup.exe:Windows' '*.dmg:macOS'; do
  glob=${entry%%:*}
  label=${entry#*:}
  for file in "$artifacts"/$glob; do
    [ -e "$file" ] || continue
    printf '| %s | %s%s%s |\n' "$label" '`' "$(basename "$file")" '`'
  done
done

printf '\nVerify a download against %sSHA256SUMS%s. Every artifact carries a GitHub\n' '`' '`'
printf 'build-provenance attestation.\n\n'

printf '## Server image\n\n'
printf '%s\n%s@%s\n%s\n\n' "$fence" "$image" "$digest" "$fence"
printf 'Production pins this digest, never a mutable tag.\n\n'

gh api --method POST "repos/${repository}/releases/generate-notes" \
  -f tag_name="$tag" --jq '.body'
