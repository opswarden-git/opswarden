#!/usr/bin/env bash
set -euo pipefail

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
    echo "source hygiene: legacy file '$file' grew from ${base_lines} to ${current_lines} lines; files above ${MAX_SOURCE_LINES} lines may only shrink" >&2
    failures=1
  fi
done < <(git diff --name-only --diff-filter=AM "$merge_base" --)

if rg --line-number --glob '*.{ts,tsx,js,jsx,mjs,cjs}' \
  '\bas any\b|@ts-ignore|@ts-nocheck' client-web client-desktop; then
  echo "source hygiene: unsafe TypeScript bypass detected" >&2
  failures=1
fi

if git diff -M --diff-filter=D --name-only "$merge_base" -- \
  | rg --quiet '(^server/tests/|\.test\.(ts|tsx|js|jsx)$|^tooling/e2e/)'; then
  echo "source hygiene: deleting a test requires a dedicated replacement/refactor PR" >&2
  failures=1
fi

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "Source hygiene passed (new files <= ${MAX_SOURCE_LINES} lines; legacy oversized files did not grow; no unsafe TypeScript bypasses)."
