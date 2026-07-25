#!/usr/bin/env bash
set -euo pipefail

test_log=${1:-target/test-results/rust.log}
summary_file=${GITHUB_STEP_SUMMARY:-/dev/stdout}

if [[ ! -f "$test_log" ]]; then
  echo "Missing Rust test log: $test_log" >&2
  exit 1
fi

read -r passed failed ignored < <(
  awk '
    /^test result:/ {
      for (i = 1; i <= NF; i++) {
        if ($i == "passed;") passed += $(i - 1)
        if ($i == "failed;") failed += $(i - 1)
        if ($i == "ignored;") ignored += $(i - 1)
      }
    }
    END { print passed + 0, failed + 0, ignored + 0 }
  ' "$test_log"
)

if (( passed == 0 || failed != 0 )); then
  echo "Unexpected Rust test totals: passed=$passed failed=$failed ignored=$ignored" >&2
  exit 1
fi

{
  echo "## Backend · Tests"
  echo
  echo "| Metric | Result |"
  echo "| --- | ---: |"
  echo "| Tests | ${passed} passed |"
  echo "| Failed | ${failed} |"
  echo "| Ignored | ${ignored} |"
} >> "$summary_file"
