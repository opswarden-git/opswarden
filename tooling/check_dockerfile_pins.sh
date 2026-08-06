#!/usr/bin/env bash
set -euo pipefail

# A missing tool must fail loudly. This check exists because it did not: these
# gates called `rg`, the runner image has no ripgrep, and every call site sat
# inside an `if` or a process substitution — so "command not found" read as
# "nothing matched" and the policy reported success without ever running.
for tool in grep find awk; do
  command -v "$tool" >/dev/null || {
    echo "Docker pin policy: required tool '$tool' is not installed" >&2
    exit 1
  }
done

failures=0
while IFS= read -r dockerfile; do
  while IFS= read -r from_line; do
    image=$(awk '{print $2}' <<<"$from_line")
    if [[ ! "$image" =~ @sha256:[0-9a-f]{64}$ ]]; then
      echo "Docker pin policy: mutable base '$image' in $dockerfile" >&2
      failures=1
    fi
  done < <(grep -E '^FROM[[:space:]]+' "$dockerfile" || true)
done < <(find . -type f -name Dockerfile -not -path './target/*' -not -path './node_modules/*' | sort)

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "Docker pin policy passed (every base image uses an immutable sha256 digest)."
