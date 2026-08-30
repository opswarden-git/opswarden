#!/bin/sh
set -eu

readonly DOCS_IMAGE="squidfunk/mkdocs-material:9.7.7@sha256:51b87149d227691486b5f08993d28c65ca7e4990010664b697265b8e6fcd5287"
REPOSITORY_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
readonly REPOSITORY_ROOT

workspace=$(mktemp -d)
cleanup() {
  rm -rf "$workspace"
}
trap cleanup EXIT INT TERM

wiki_source=${OPSWARDEN_WIKI_SOURCE_DIR:-}
if [ -z "$wiki_source" ]; then
  wiki_source="$workspace/wiki"
  git clone --depth 1 --quiet \
    https://github.com/opswarden-git/opswarden.wiki.git "$wiki_source"
fi

test -f "$wiki_source/mkdocs.yml"
test -d "$wiki_source/docs-source"

portal="$workspace/portal"
mkdir -p "$portal/docs"
cp "$wiki_source/mkdocs.yml" "$portal/mkdocs.yml"
cp -R "$wiki_source/docs-source/." "$portal/docs/"
rm -f "$portal/docs/assets/logo.png"
cp "$REPOSITORY_ROOT/client-web/public/assets/heroicon.png" "$portal/docs/assets/logo.png"

# Root contracts are canonical and included by MkDocs snippets.
cp "$REPOSITORY_ROOT/README.md" "$portal/README.md"
for contract in DESIGN_SYSTEM.md UI_GUIDELINES.md HOWTOCONTRIBUTE.md WEBSOCKET_SPEC.md INTEGRATION_GUIDE.md; do
  cp "$REPOSITORY_ROOT/docs/$contract" "$portal/$contract"
done

OPSWARDEN_API_URL=${OPSWARDEN_API_URL:-https://api.opswarden.dev} \
  OPSWARDEN_INVENTORY_DOCS_DIR="$portal/docs/inventory" \
  node "$REPOSITORY_ROOT/tooling/inventory/build.mjs"

# Strip local dev file URIs from markdown docs so link checkers pass cleanly
find "$portal" -type f -name '*.md' -exec sed -i -E 's|file:///[^)]*/opswarden/|https://github.com/opswarden-git/opswarden/tree/main/|g' {} +

docker run --rm \
  --user "$(id -u):$(id -g)" \
  --volume "$portal:/docs" \
  --workdir /docs \
  "$DOCS_IMAGE" build --strict

output="$REPOSITORY_ROOT/site"
rm -rf "$output"
mkdir -p "$output"
cp -R "$portal/site/." "$output/"
touch "$output/.nojekyll"

printf 'documentation: wiki + contracts + inventory -> %s\n' "$output"
