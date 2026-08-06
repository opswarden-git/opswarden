#!/usr/bin/env bash
set -euo pipefail

# A missing tool must fail loudly. This check exists because it did not: these
# gates called `rg`, the runner image has no ripgrep, and every call site sat
# inside an `if` or a process substitution — so "command not found" read as
# "nothing matched" and the policy reported success without ever running.
for tool in git grep sed; do
  command -v "$tool" >/dev/null || {
    echo "migration policy: required tool '$tool' is not installed" >&2
    exit 1
  }
done

base_ref=${1:-origin/main}
if ! git rev-parse --verify --quiet "${base_ref}^{commit}" >/dev/null; then
  echo "migration policy: base commit '$base_ref' is unavailable" >&2
  exit 2
fi

merge_base=$(git merge-base "$base_ref" HEAD)
failures=0
readonly destructive_pattern='\b(drop[[:space:]]+(table|column|constraint|function|trigger|index)|truncate|alter[[:space:]]+table.*rename|alter[[:space:]]+column.*type|set[[:space:]]+not[[:space:]]+null)\b'

while IFS= read -r file; do
  [[ -n "$file" ]] || continue
  status=$(git diff --name-status "$merge_base" -- "$file" | awk 'NR == 1 { print $1 }')
  if [[ "$status" == M* ]]; then
    echo "migration policy: applied migration '$file' is immutable; add a new migration" >&2
    failures=1
    continue
  fi

  phase=$(sed -nE 's/^-- opswarden: migration-phase=(expand|backfill|contract)$/\1/p' "$file")
  if [[ -z "$phase" || "$phase" == *$'\n'* ]]; then
    echo "migration policy: '$file' needs exactly one expand, backfill or contract phase marker" >&2
    failures=1
    continue
  fi

  if [[ "$phase" != contract ]] && grep --quiet --ignore-case --extended-regexp "$destructive_pattern" "$file"; then
    echo "migration policy: destructive SQL is forbidden in '$phase' migration '$file'" >&2
    failures=1
  fi
done < <(
  git diff --name-only --diff-filter=AM "$merge_base" -- 'server/migrations/*.sql'
)

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "Migration policy passed (immutable, phase-labelled expand/contract workflow)."
