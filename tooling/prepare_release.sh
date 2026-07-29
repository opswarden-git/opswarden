#!/usr/bin/env bash
set -euo pipefail

# Prepare a release: rewrite every version-bearing file, refresh the lockfiles
# that embed the version, verify consistency, then commit and tag.
#
# The tag is created locally and never pushed: pushing it is what triggers the
# Release workflow, so that step stays an explicit decision.

usage() {
  echo "usage: ${0##*/} <version>" >&2
  echo "  e.g. ${0##*/} 1.0.7" >&2
}

version=${1:-}
if [ -z "$version" ]; then
  usage
  exit 1
fi
version=${version#v}

if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid semantic version: $version" >&2
  exit 1
fi

root="$(git rev-parse --show-toplevel)"
cd "$root"

if [ -n "$(git status --porcelain)" ]; then
  echo "Working tree is not clean; commit or stash first." >&2
  exit 1
fi

branch=$(git rev-parse --abbrev-ref HEAD)
if [ "$branch" != main ]; then
  echo "Releases are cut from main, not $branch." >&2
  exit 1
fi

if git rev-parse -q --verify "refs/tags/v$version" >/dev/null; then
  echo "Tag v$version already exists." >&2
  exit 1
fi

current=$(awk -F '"' '/^version = "/ { print $2; exit }' server/Cargo.toml)
if [ "$current" = "$version" ]; then
  echo "server/Cargo.toml is already at $version; nothing to bump." >&2
  exit 1
fi

echo ">> Bumping $current -> $version"

for tool in jq awk; do
  command -v "$tool" >/dev/null || {
    echo "$tool is required" >&2
    exit 1
  }
done

set_toml_version() {
  local file=$1
  # Only the first `version = "..."` line, which is the package's own version;
  # dependency pins further down must not be touched.
  awk -v v="$version" '
    !done && /^version = "/ { print "version = \"" v "\""; done = 1; next }
    { print }
  ' "$file" >"$file.tmp"
  mv "$file.tmp" "$file"
}

set_json_version() {
  local file=$1
  jq --arg v "$version" '.version = $v' "$file" >"$file.tmp"
  mv "$file.tmp" "$file"
}

# Rewrite the version of one package inside a Cargo.lock. Third-party crates
# legitimately share our version number (new_debug_unreachable and same-file are
# both at 1.0.6 today), so the entry is located by name, never by value.
set_lock_version() {
  local file=$1 pkg=$2
  awk -v pkg="$pkg" -v v="$version" '
    $0 == "name = \"" pkg "\"" { found = 1; print; next }
    found && /^version = "/ { print "version = \"" v "\""; found = 0; next }
    { print }
  ' "$file" >"$file.tmp"
  mv "$file.tmp" "$file"
}

set_toml_version server/Cargo.toml
set_toml_version client-desktop/src-tauri/Cargo.toml
set_json_version client-web/package.json
set_json_version client-desktop/package.json
set_json_version client-desktop/src-tauri/tauri.conf.json

# The lockfiles embed the version too. They are rewritten in place rather than
# regenerated: `cargo update` on the desktop crate would require the Tauri
# toolchain, which the server workspace deliberately excludes, and `npm install`
# would reach the network. verify_release_version.sh below is the safety net.
set_lock_version Cargo.lock opswarden-server
set_lock_version client-desktop/src-tauri/Cargo.lock opswarden-desktop
jq --arg v "$version" '
  .packages["client-web"].version = $v
  | .packages["client-desktop"].version = $v
' package-lock.json >package-lock.json.tmp
mv package-lock.json.tmp package-lock.json

./tooling/verify_release_version.sh "$version"

git add -A
git commit --quiet --message "Release v$version"
# Annotated, not lightweight: `git push --follow-tags` silently skips lightweight
# tags, so the release would never trigger and the push would look like it worked.
git tag --annotate "v$version" --message "Release v$version"

echo
echo ">> Release v$version prepared on main."
echo "   Review with: git show --stat HEAD"
echo "   Publish with: git push origin main --follow-tags"
