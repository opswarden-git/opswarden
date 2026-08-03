#!/usr/bin/env bash
set -euo pipefail

exception_expires=2026-10-31
today=${OPSWARDEN_AUDIT_DATE:-$(date -u +%F)}

# Runtime dependencies have no exception: every high or critical advisory blocks.
npm audit --omit=dev --audit-level=high

audit_report=$(mktemp)
trap 'rm -f "$audit_report"' EXIT
npm audit --json > "$audit_report" || true

critical_count=$(jq -r '.metadata.vulnerabilities.critical' "$audit_report")
if (( critical_count != 0 )); then
  echo "npm audit found $critical_count critical vulnerability/vulnerabilities" >&2
  exit 1
fi

allowed_chain='[
  "@eslint/config-array",
  "@eslint/eslintrc",
  "brace-expansion",
  "eslint",
  "eslint-config-next",
  "eslint-plugin-import",
  "eslint-plugin-jsx-a11y",
  "eslint-plugin-react",
  "minimatch"
]'
unexpected=$(jq -r --argjson allowed "$allowed_chain" '
  [(.vulnerabilities | keys[]) as $key | select($allowed | index($key) | not) | $key]
  | join(",")
' "$audit_report")
if [[ -n "$unexpected" ]]; then
  echo "npm audit found non-allowlisted development advisories: $unexpected" >&2
  exit 1
fi

if jq -e '.vulnerabilities["brace-expansion"]' "$audit_report" >/dev/null; then
  advisory_urls=$(jq -c '[
    .vulnerabilities[].via[]
    | select(type == "object")
    | .url
  ] | unique' "$audit_report")
  if [[ "$advisory_urls" != '["https://github.com/advisories/GHSA-mh99-v99m-4gvg"]' ]]; then
    echo "The ESLint dependency chain contains an advisory outside the reviewed exception" >&2
    exit 1
  fi
  if [[ "$today" > "$exception_expires" ]]; then
    echo "The brace-expansion development-only exception expired on $exception_expires" >&2
    exit 1
  fi
  jq -e '.devDependencies["brace-expansion"] == "1.1.18"' package.json >/dev/null || {
    echo "The brace-expansion exception is valid only for the pinned ESLint 9 compatibility package" >&2
    exit 1
  }
  echo "Accepted development-only GHSA-mh99-v99m-4gvg until $exception_expires; production dependencies are clean."
else
  echo "All npm dependencies are free of known high or critical vulnerabilities."
fi
