#!/usr/bin/env bash
set -euo pipefail

# A missing tool must fail loudly. This check exists because it did not: these
# gates called `rg`, the runner image has no ripgrep, and every call site sat
# inside an `if` or a process substitution — so "command not found" read as
# "nothing matched" and the policy reported success without ever running.
for tool in git grep xargs awk; do
  command -v "$tool" >/dev/null || {
    echo "source hygiene: required tool '$tool' is not installed" >&2
    exit 1
  }
done

readonly MAX_SOURCE_LINES=500

base_ref=${1:-origin/main}
if ! git rev-parse --verify --quiet "${base_ref}^{commit}" >/dev/null; then
  echo "source hygiene: base commit '$base_ref' is unavailable" >&2
  exit 2
fi

merge_base=$(git merge-base "$base_ref" HEAD)
failures=0

is_source_file() {
  case "$1" in
    *.rs | *.ts | *.tsx | *.js | *.jsx | *.mjs | *.cjs | *.sh) return 0 ;;
    *) return 1 ;;
  esac
}

line_count_at() {
  local revision=$1 file=$2
  git show "${revision}:${file}" 2>/dev/null | awk 'END { print NR + 0 }'
}

while IFS= read -r file; do
  [[ -n "$file" && -f "$file" ]] || continue
  is_source_file "$file" || continue

  current_lines=$(awk 'END { print NR + 0 }' "$file")
  base_lines=$(line_count_at "$merge_base" "$file" || true)

  if [[ -z "$base_lines" && "$current_lines" -gt "$MAX_SOURCE_LINES" ]]; then
    echo "source hygiene: new file '$file' has ${current_lines} lines (maximum ${MAX_SOURCE_LINES})" >&2
    failures=1
  elif [[ -n "$base_lines" && "$current_lines" -gt "$MAX_SOURCE_LINES" && "$current_lines" -gt "$base_lines" ]]; then
    echo "source hygiene: '$file' grew from ${base_lines} to ${current_lines} lines (maximum ${MAX_SOURCE_LINES})" >&2
    failures=1
  fi
done < <(git diff --name-only --diff-filter=AM "$merge_base" --)

# Enumerated through `git ls-files` rather than scanned recursively: a bare
# `grep -r` would walk node_modules and .next, where these patterns are
# everywhere and none of them are ours.
bypasses=$(
  git ls-files -- 'client-web' 'client-desktop' |
    grep --extended-regexp '\.(ts|tsx|js|jsx|mjs|cjs)$' |
    while IFS= read -r source_file; do
      # `git ls-files` includes paths deleted from the working tree until the
      # change is committed. Skip those paths instead of emitting misleading
      # grep errors on every legitimate source deletion.
      [[ -f "$source_file" ]] || continue
      grep --line-number --with-filename --extended-regexp \
        '\bas any\b|@ts-ignore|@ts-nocheck' "$source_file" || true
    done
)
if [[ -n "$bypasses" ]]; then
  echo "$bypasses" >&2
  echo "source hygiene: unsafe TypeScript bypass detected" >&2
  failures=1
fi

# Losing coverage of code that still exists is the risk here, so a deleted test
# only passes when the source it covered is deleted in the same change. Removing
# a feature legitimately takes its tests with it; dropping the tests while the
# handlers stay behind is what this must keep catching.
deleted=$(git diff -M --diff-filter=D --name-only "$merge_base" --)
deleted_tests=$(grep --extended-regexp '(^server/tests/|\.test\.(ts|tsx|js|jsx)$|^tooling/e2e/)' <<<"$deleted" || true)
if [[ -n "$deleted_tests" ]]; then
  deleted_sources=$(
    grep --invert-match --extended-regexp \
      '(^server/tests/|\.test\.(ts|tsx|js|jsx)$|^tooling/e2e/)' <<<"$deleted" |
      grep --extended-regexp '\.(rs|ts|tsx|js|jsx|mjs|cjs)$' || true
  )
  if [[ -z "$deleted_sources" ]]; then
    echo "source hygiene: deleting a test requires a dedicated replacement/refactor PR" >&2
    failures=1
  else
    echo "source hygiene: tests removed alongside the source they covered:"
    while IFS= read -r removed_test; do
      echo "  - $removed_test"
    done <<<"$deleted_tests"
  fi
fi

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "Source hygiene passed (changed files <= ${MAX_SOURCE_LINES} lines; no unsafe TypeScript bypasses)."
