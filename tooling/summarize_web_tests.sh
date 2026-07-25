#!/usr/bin/env bash
set -euo pipefail

test_log=${1:-target/test-results/web.log}
coverage_file=${2:-client-web/coverage/coverage-summary.json}
summary_file=${GITHUB_STEP_SUMMARY:-/dev/stdout}

if [[ ! -f "$test_log" || ! -f "$coverage_file" ]]; then
  echo "Missing Vitest log or coverage summary" >&2
  exit 1
fi

clean_log=$(mktemp)
trap 'rm -f "$clean_log"' EXIT
sed -E 's/\x1B\[[0-9;]*[[:alpha:]]//g' "$test_log" > "$clean_log"

file_line=$(awk '/Test Files[[:space:]]+[0-9]+ passed/ { line = $0 } END { print line }' "$clean_log")
test_line=$(awk '/Tests[[:space:]]+[0-9]+ passed/ { line = $0 } END { print line }' "$clean_log")
files_passed=$(awk '{ for (i=1; i<=NF; i++) if ($i == "Files") { print $(i+1); exit } }' <<< "$file_line")
tests_passed=$(awk '{ for (i=1; i<=NF; i++) if ($i == "Tests") { print $(i+1); exit } }' <<< "$test_line")
files_total=$(sed -E 's/.*\(([0-9]+)\).*/\1/' <<< "$file_line")
tests_total=$(sed -E 's/.*\(([0-9]+)\).*/\1/' <<< "$test_line")

if [[ -z "$files_passed" || -z "$tests_passed" ]]; then
  echo "Could not parse the Vitest result counts" >&2
  exit 1
fi

lines=$(jq -r '.total.lines.pct' "$coverage_file")
branches=$(jq -r '.total.branches.pct' "$coverage_file")
functions=$(jq -r '.total.functions.pct' "$coverage_file")
statements=$(jq -r '.total.statements.pct' "$coverage_file")

{
  echo "## Web · Tests"
  echo
  echo "| Metric | Result |"
  echo "| --- | ---: |"
  echo "| Test files | ${files_passed}/${files_total} passed |"
  echo "| Tests | ${tests_passed}/${tests_total} passed |"
  echo "| Line coverage | ${lines}% (minimum 70%) |"
  echo "| Branch coverage | ${branches}% (minimum 55%) |"
  echo "| Function coverage | ${functions}% (minimum 65%) |"
  echo "| Statement coverage | ${statements}% (minimum 65%) |"
} >> "$summary_file"
