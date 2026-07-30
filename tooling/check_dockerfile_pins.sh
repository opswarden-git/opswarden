#!/usr/bin/env bash
set -euo pipefail

failures=0
while IFS= read -r dockerfile; do
  while IFS= read -r from_line; do
    image=$(awk '{print $2}' <<<"$from_line")
    if [[ ! "$image" =~ @sha256:[0-9a-f]{64}$ ]]; then
      echo "Docker pin policy: mutable base '$image' in $dockerfile" >&2
      failures=1
    fi
  done < <(rg --no-filename '^FROM[[:space:]]+' "$dockerfile")
done < <(find . -type f -name Dockerfile -not -path './target/*' -not -path './node_modules/*' | sort)

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "Docker pin policy passed (every base image uses an immutable sha256 digest)."
