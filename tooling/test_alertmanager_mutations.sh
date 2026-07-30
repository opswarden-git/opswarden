#!/usr/bin/env bash
set -euo pipefail

readonly REQUIRED_VERSION="27.1.0"

if ! cargo mutants --version >/dev/null 2>&1; then
  echo "cargo-mutants ${REQUIRED_VERSION} is required." >&2
  echo "Install it with: cargo install --locked cargo-mutants --version ${REQUIRED_VERSION}" >&2
  exit 2
fi

installed_version="$(cargo mutants --version | awk '{print $2}')"
if [[ "$installed_version" != "$REQUIRED_VERSION" ]]; then
  echo "Expected cargo-mutants ${REQUIRED_VERSION}, found ${installed_version}." >&2
  exit 2
fi

# These modules contain the Alertmanager trust boundary: parsing/idempotence
# and the bounded accepted/rejected/duplicate/ignored/failed counters.
mkdir -p target
cargo mutants \
  --package opswarden-server \
  --file server/src/adapters/webhook/alertmanager.rs \
  --file server/src/adapters/metrics.rs \
  --jobs 2 \
  --timeout 120 \
  --output target/mutants-alertmanager \
  -- --lib
