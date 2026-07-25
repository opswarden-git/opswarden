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

versions=(
  "$server_version"
  "$web_version"
  "$desktop_npm_version"
  "$desktop_rust_version"
  "$tauri_version"
)

expected=${requested_version:-$server_version}
if [[ ! "$expected" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid semantic version: $expected" >&2
  exit 1
fi

labels=(server web desktop-npm desktop-rust tauri)
for index in "${!versions[@]}"; do
  if [[ "${versions[$index]}" != "$expected" ]]; then
    echo "${labels[$index]} version ${versions[$index]} does not match $expected" >&2
    exit 1
  fi
done

echo "Release version is consistent across every package: $expected"
