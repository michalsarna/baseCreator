#!/bin/bash
# Renders the app icon from its SVG source: PNGs of every size, a Windows .ico and a macOS .icns.
# Needs inkscape, python3 and (for the .icns) macOS iconutil.
# Usage: tools/make-icons.sh assets/icons/millcreator.svg millcreator
set -euo pipefail
svg="$1"; name="$2"
out="$(cd "$(dirname "$svg")" && pwd)"
for s in 16 24 32 48 64 96 128 256 512 1024; do
  inkscape "$svg" -o "$out/icon-$s.png" -w "$s" -h "$s" >/dev/null 2>&1
done
python3 - "$out" "$out/../$name.ico" <<'PY'
import struct, sys
src, dst = sys.argv[1], sys.argv[2]
sizes = [16, 32, 48, 64, 128, 256]
imgs = [open(f"{src}/icon-{s}.png", "rb").read() for s in sizes]
head = struct.pack("<HHH", 0, 1, len(sizes))
off = 6 + 16 * len(sizes)
dirs = b""
for s, d in zip(sizes, imgs):
    dirs += struct.pack("<BBBBHHII", s % 256, s % 256, 0, 0, 1, 32, len(d), off)
    off += len(d)
open(dst, "wb").write(head + dirs + b"".join(imgs))
PY
if command -v iconutil >/dev/null; then
  set_dir="$(mktemp -d)/$name.iconset"; mkdir -p "$set_dir"
  for pair in 16:16x16 32:16x16@2x 32:32x32 64:32x32@2x 128:128x128 256:128x128@2x 256:256x256 512:256x256@2x 512:512x512 1024:512x512@2x; do
    cp "$out/icon-${pair%%:*}.png" "$set_dir/icon_${pair#*:}.png"
  done
  iconutil -c icns "$set_dir" -o "$out/../$name.icns"
fi
