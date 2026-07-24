#!/bin/sh
set -eu

base_url=${1:-http://localhost:8081}
artifact=${2:-artifacts/client.AppImage}

[ -x "$artifact" ] || {
  echo "Missing executable Compose artifact: $artifact" >&2
  exit 1
}

smoke_dir=$(mktemp -d)
trap 'rm -rf "$smoke_dir"' EXIT HUP INT TERM
download="$smoke_dir/client.AppImage"
headers="$smoke_dir/headers"

curl --fail --silent --show-error \
  --dump-header "$headers" \
  --output "$download" \
  "$base_url/client.AppImage"

grep -Eq '^HTTP/[0-9.]+ 200 ' "$headers"
grep -Eiq '^Content-Type: application/octet-stream' "$headers"
cmp "$artifact" "$download"

signature=$(od -An -tx1 -j8 -N3 "$download" | tr -d ' \n')
[ "$signature" = "414902" ] || {
  echo "Downloaded file is not a Type 2 AppImage: $signature" >&2
  exit 1
}

echo "Compose AppImage download smoke test passed"
echo "  bytes: $(stat -c %s "$download")"
echo "  sha256: $(sha256sum "$download" | awk '{print $1}')"
