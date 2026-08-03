#!/usr/bin/env bash
set -euo pipefail

# Re-downloads every AppImage helper preseeded by client-desktop/Dockerfile and
# checks it against the digest pinned there.
#
# Why this exists: the Compose desktop image is the only build path the CI never
# exercises (both ci.yml and release.yml start client_web with `--no-deps`, which
# skips the client_desktop service). A pinned upstream asset can therefore drift
# out from under `just up` while every workflow stays green -- which is exactly
# what happened when linuxdeploy-plugin-appimage was pinned to the rolling
# `continuous` tag: the digest stopped matching and `just up` failed on a fresh
# clone for days without a single red run.
#
# The URL/digest pairs are parsed straight out of the Dockerfile so this check
# can never disagree with what the build actually downloads.

dockerfile=${1:-client-desktop/Dockerfile}

[[ -r "$dockerfile" ]] || {
  echo "Cannot read Dockerfile: $dockerfile" >&2
  exit 1
}

# `-o /root/.cache/tauri/<name>` is preceded by the URL curl fetches; the digest
# table lists `<sha256>  /root/.cache/tauri/<name>` one entry per line.
declare -A url_of digest_of
current_url=""

while IFS= read -r line; do
  token=${line#"${line%%[![:space:]]*}"}
  token=${token%%[[:space:]]*}
  token=${token#\"}
  token=${token%\"}

  case "$token" in
    http://* | https://*) current_url=$token ;;
  esac

  if [[ "$line" =~ -o[[:space:]]+/root/\.cache/tauri/([A-Za-z0-9._-]+) ]]; then
    url_of["${BASH_REMATCH[1]}"]=$current_url
  fi

  if [[ "$line" =~ ([0-9a-f]{64})[[:space:]]+/root/\.cache/tauri/([A-Za-z0-9._-]+) ]]; then
    digest_of["${BASH_REMATCH[2]}"]=${BASH_REMATCH[1]}
  fi
done <"$dockerfile"

(("${#digest_of[@]}" > 0)) || {
  echo "No pinned digests found in $dockerfile -- the parser is out of date." >&2
  exit 1
}

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT HUP INT TERM

failures=0
for name in "${!digest_of[@]}"; do
  url=${url_of[$name]:-}
  expected=${digest_of[$name]}

  if [[ -z "$url" ]]; then
    echo "::error title=Unpinned asset::$name has a digest but no download URL" >&2
    failures=1
    continue
  fi

  # The GitHub asset-ID endpoint needs both headers; the plain release and raw
  # hosts ignore them.
  if ! curl -fL --retry 5 --retry-delay 2 --retry-all-errors -sS \
    -H "Accept: application/octet-stream" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    "$url" -o "$workdir/$name"; then
    echo "::error title=Download failed::$name is no longer reachable at $url" >&2
    failures=1
    continue
  fi

  actual=$(sha256sum "$workdir/$name" | awk '{print $1}')
  if [[ "$actual" != "$expected" ]]; then
    echo "::error title=Stale pin::$name drifted upstream" >&2
    echo "  url:      $url" >&2
    echo "  pinned:   $expected" >&2
    echo "  upstream: $actual" >&2
    failures=1
    continue
  fi

  echo "ok  $name  $expected"
done

if ((failures != 0)); then
  echo >&2
  echo "The Compose desktop build would fail on a fresh clone (\`just up\`)." >&2
  exit 1
fi

echo "All ${#digest_of[@]} pinned desktop assets match $dockerfile."
