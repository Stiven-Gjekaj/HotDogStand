#!/bin/sh
# Makes the icon files of the application from assets/hotdogstand.svg.
#
# Run it after a change to the logo, then commit the files that it writes.
# The build does not run it, so a person who builds the application needs
# none of these tools.
#
# It needs rsvg-convert and python3. On macOS it also makes the .icns file
# with iconutil, which only macOS has.
set -eu

cd "$(dirname "$0")/.."
svg=assets/hotdogstand.svg
out=assets/icons
mkdir -p "$out/png"

for size in 16 24 32 48 64 128 256 512 1024; do
    rsvg-convert -w "$size" -h "$size" "$svg" -o "$out/png/$size.png"
done

# Windows: an .ico file that holds the PNG images. Windows Vista and newer
# read PNG images inside an .ico file.
python3 - "$out" <<'PY'
import struct, sys
out = sys.argv[1]
sizes = [16, 24, 32, 48, 64, 128, 256]
images = [open(f"{out}/png/{s}.png", "rb").read() for s in sizes]
header = struct.pack("<HHH", 0, 1, len(sizes))
offset = 6 + 16 * len(sizes)
entries = b""
for size, data in zip(sizes, images):
    side = 0 if size == 256 else size  # 0 means 256 in an .ico entry.
    entries += struct.pack("<BBBBHHII", side, side, 0, 0, 1, 32, len(data), offset)
    offset += len(data)
with open(f"{out}/hotdogstand.ico", "wb") as f:
    f.write(header + entries + b"".join(images))
PY

# macOS: an .icns file from an iconset directory.
if command -v iconutil >/dev/null 2>&1; then
    set_dir=$(mktemp -d)/hotdogstand.iconset
    mkdir -p "$set_dir"
    for size in 16 32 128 256 512; do
        cp "$out/png/$size.png" "$set_dir/icon_${size}x${size}.png"
        cp "$out/png/$((size * 2)).png" "$set_dir/icon_${size}x${size}@2x.png"
    done
    iconutil -c icns "$set_dir" -o "$out/hotdogstand.icns"
    rm -r "$(dirname "$set_dir")"
else
    echo "iconutil is not here, so the .icns file stays as it is." >&2
fi

echo "Wrote the icons to $out."
