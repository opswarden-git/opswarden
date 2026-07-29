#!/usr/bin/env bash
set -euo pipefail

requested_version=${1:-}
if [[ -n "$requested_version" ]]; then
  requested_version=${requested_version#v}
fi

server_version=$(awk -F '"' '/^version = "/ { print $2; exit }' server/Cargo.toml)
desktop_rust_version=$(awk -F '"' '/^version = "/ { print $2; exit }' client-desktop/src-tauri/Cargo.toml)
web_version=$(jq -r .version client-web/package.json)
desktop_npm_version=$(jq -r .version client-desktop/package.json)
tauri_version=$(jq -r .version client-desktop/src-tauri/tauri.conf.json)

# The lockfiles embed the package version too. They are not part of the manifest
# rewrite, so a stale one silently dirties the working tree during a release
# build instead of failing here.
lock_version() {
  awk -v pkg="$2" '
    $0 == "name = \"" pkg "\"" { want = 1; next }
    want && /^version = "/ { split($0, parts, "\""); print parts[2]; exit }
  ' "$1"
}

server_lock_version=$(lock_version Cargo.lock opswarden-server)
desktop_lock_version=$(lock_version client-desktop/src-tauri/Cargo.lock opswarden-desktop)
web_npm_lock_version=$(jq -r '.packages["client-web"].version' package-lock.json)
desktop_npm_lock_version=$(jq -r '.packages["client-desktop"].version' package-lock.json)

versions=(
  "$server_version"
  "$web_version"
  "$desktop_npm_version"
  "$desktop_rust_version"
  "$tauri_version"
  "$server_lock_version"
  "$desktop_lock_version"
  "$web_npm_lock_version"
  "$desktop_npm_lock_version"
)

expected=${requested_version:-$server_version}
if [[ ! "$expected" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid semantic version: $expected" >&2
  exit 1
fi

labels=(
  server
  web
  desktop-npm
  desktop-rust
  tauri
  server-lock
  desktop-lock
  web-npm-lock
  desktop-npm-lock
)
for index in "${!versions[@]}"; do
  if [[ "${versions[$index]}" != "$expected" ]]; then
    echo "${labels[$index]} version ${versions[$index]} does not match $expected" >&2
    exit 1
  fi
done

echo "Release version is consistent across every package and lockfile: $expected"
