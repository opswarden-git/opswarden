#!/bin/sh
set -eu

npm run tauri build -- --bundles deb,appimage

set -- src-tauri/target/release/bundle/deb/*.deb
[ "$#" -eq 1 ] && [ -f "$1" ] || {
  echo "Expected exactly one desktop .deb artifact" >&2
  exit 1
}
deb_artifact=$1

set -- src-tauri/target/release/bundle/appimage/*.AppImage
[ "$#" -eq 1 ] && [ -f "$1" ] || {
  echo "Expected exactly one desktop AppImage artifact" >&2
  exit 1
}
appimage_artifact=$1

mkdir -p /out
install -m 0644 "$deb_artifact" /out/OpsWarden_amd64.deb
install -m 0755 "$appimage_artifact" /out/client.AppImage

sh scripts/verify-appimage.sh /out/client.AppImage

echo "Desktop artifacts ready:"
echo "  /out/OpsWarden_amd64.deb"
echo "  /out/client.AppImage"
