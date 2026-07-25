#!/usr/bin/env bash
set -euo pipefail

coverage_file=${1:-target/tarpaulin/coverage.json}
summary_file=${2:-target/tarpaulin/source-only-summary.json}
minimum_percent=${3:-70}

if [[ ! -f "$coverage_file" ]]; then
  echo "Missing Tarpaulin JSON report: $coverage_file" >&2
  exit 1
fi

unexpected_paths=$(jq '[
  .traces | keys[]
  | select(test("(^|/)server/src/.*\\.rs$") | not)
] | length' "$coverage_file")

if (( unexpected_paths > 0 )); then
  echo "Tarpaulin report contains $unexpected_paths path(s) outside server/src" >&2
  jq -r '.traces | keys[] | select(test("(^|/)server/src/.*\\.rs$") | not)' \
    "$coverage_file" >&2
  exit 1
fi

if jq -e '.traces | keys[] | test("(^|/)server/src/main\\.rs$")' \
  "$coverage_file" >/dev/null; then
  echo "Tarpaulin report unexpectedly includes server/src/main.rs" >&2
  exit 1
fi

mkdir -p "$(dirname "$summary_file")"
summary_tmp="${summary_file}.tmp"
trap 'rm -f "$summary_tmp"' EXIT

jq --argjson minimum "$minimum_percent" '
  [.traces | to_entries[] | .value[]] as $lines
  | ($lines | length) as $total
  | ([$lines[] | select((.stats.Line // 0) > 0)] | length) as $covered
  | if $total == 0 then error("Tarpaulin source-only report contains no lines") else
      {
        scope: "server/src/**/*.rs excluding main.rs and test functions",
        covered_lines: $covered,
        total_lines: $total,
        line_coverage_percent: (($covered * 10000 / $total | round) / 100),
        minimum_percent: $minimum
      }
    end
' "$coverage_file" > "$summary_tmp"

coverage_percent=$(jq -r '.line_coverage_percent' "$summary_tmp")
if ! awk -v actual="$coverage_percent" -v minimum="$minimum_percent" \
  'BEGIN { exit !(actual >= minimum) }'; then
  echo "Backend source-only coverage ${coverage_percent}% is below ${minimum_percent}%" >&2
  exit 1
fi

mv "$summary_tmp" "$summary_file"
trap - EXIT

jq -r '
  "Backend source-only coverage: \(.covered_lines)/\(.total_lines) lines "
  + "(\(.line_coverage_percent)%, minimum \(.minimum_percent)%)"
' "$summary_file"
