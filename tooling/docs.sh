#!/bin/sh
set -eu

readonly DOCS_IMAGE="squidfunk/mkdocs-material:9.7.7@sha256:51b87149d227691486b5f08993d28c65ca7e4990010664b697265b8e6fcd5287"
REPOSITORY_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
readonly REPOSITORY_ROOT

run_mkdocs() {
  docker run --rm \
    --user "$(id -u):$(id -g)" \
    --volume "${REPOSITORY_ROOT}:/docs" \
    --workdir /docs \
    "$DOCS_IMAGE" "$@"
}

case "${1:-build}" in
  build)
    run_mkdocs build --strict
    ;;
  serve)
    docker run --rm \
      --user "$(id -u):$(id -g)" \
      --publish "${DOCS_PORT:-8000}:8000" \
      --volume "${REPOSITORY_ROOT}:/docs" \
      --workdir /docs \
      "$DOCS_IMAGE" serve --dev-addr 0.0.0.0:8000
    ;;
  *)
    echo "Usage: $0 [build|serve]" >&2
    exit 2
    ;;
esac
