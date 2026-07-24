#!/bin/sh
set -eu

artifact=${1:-}
[ -n "$artifact" ] && [ -f "$artifact" ] || {
  echo "Usage: $0 /path/to/client.AppImage" >&2
  exit 1
}
[ -x "$artifact" ] || {
  echo "AppImage is not executable: $artifact" >&2
  exit 1
}

# Type 2 AppImages carry the ASCII bytes AI followed by 0x02 at ELF offsets
# 8..10. A renamed archive or Debian package cannot satisfy this contract.
signature=$(od -An -tx1 -j8 -N3 "$artifact" | tr -d ' \n')
[ "$signature" = "414902" ] || {
  echo "Invalid AppImage Type 2 signature: $signature" >&2
  exit 1
}

smoke_dir=$(mktemp -d)
trap 'rm -rf "$smoke_dir"' EXIT HUP INT TERM
smoke_image="$smoke_dir/client.AppImage"
cp "$artifact" "$smoke_image"

# Build containers cannot mount AppImages through FUSE. Clear the signature on
# the disposable copy so the embedded runtime can execute its extraction path;
# the delivered artifact above remains byte-for-byte untouched.
dd if=/dev/zero bs=1 count=3 seek=8 conv=notrunc of="$smoke_image" >/dev/null 2>&1
(
  cd "$smoke_dir"
  env -u APPIMAGE_EXTRACT_AND_RUN "$smoke_image" --appimage-extract >/dev/null
)

appdir="$smoke_dir/squashfs-root"
[ -x "$appdir/AppRun" ]
[ -x "$appdir/usr/bin/opswarden-desktop" ]

if ldd "$appdir/usr/bin/opswarden-desktop" | grep -q "not found"; then
  echo "The bundled desktop executable has unresolved shared libraries" >&2
  ldd "$appdir/usr/bin/opswarden-desktop" >&2
  exit 1
fi

set +e
timeout 5 xvfb-run -a "$appdir/AppRun" >"$smoke_dir/launch.log" 2>&1
launch_status=$?
set -e
case "$launch_status" in
  0 | 124) ;;
  *)
    echo "The extracted AppImage failed its virtual-display launch smoke test" >&2
    cat "$smoke_dir/launch.log" >&2
    exit 1
    ;;
esac

echo "AppImage smoke test passed: Type 2 signature, extraction and virtual-display launch"
