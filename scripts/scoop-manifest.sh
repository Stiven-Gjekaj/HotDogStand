#!/usr/bin/env bash
# Write the Scoop manifest of one release, from the SHA256SUMS of that
# release.
#
# The manifest installs the Windows package of the release, and puts
# HotDogStand in the Start menu. A person puts it into the bucket and commits
# it there: no bot pushes to the bucket.
#
# Usage: scripts/scoop-manifest.sh <version> <SHA256SUMS> > ../scoop-bucket/bucket/hotdogstand.json

set -euo pipefail

USAGE="usage: scoop-manifest.sh <version> <SHA256SUMS>"
VERSION="${1:?$USAGE}"
SUMS="${2:?$USAGE}"

FILE="HotDogStand-windows-x86_64.zip"
SHA="$(awk -v f="$FILE" '$2 == f { print $1 }' "$SUMS")"
if [[ ! "$SHA" =~ ^[0-9a-f]{64}$ ]]; then
    echo "scoop-manifest.sh: $SUMS holds no digest for $FILE" >&2
    exit 1
fi

# Scoop puts the version where $version stands, and it hashes the file
# itself.
LATER='https://github.com/Stiven-Gjekaj/HotDogStand/releases/download/v$version'

cat <<EOF2
{
    "version": "${VERSION}",
    "description": "A ticket manager on your own computer, with the look of Windows 7",
    "homepage": "https://github.com/Stiven-Gjekaj/HotDogStand",
    "license": "MIT",
    "notes": "HotDogStand keeps your tickets in %APPDATA%\\\\HotDogStand\\\\workspace.db. An uninstall does not remove them.",
    "architecture": {
        "64bit": {
            "url": "https://github.com/Stiven-Gjekaj/HotDogStand/releases/download/v${VERSION}/${FILE}",
            "hash": "${SHA}"
        }
    },
    "extract_dir": "HotDogStand",
    "bin": [["HotDogStand.exe", "hotdogstand"]],
    "shortcuts": [["HotDogStand.exe", "HotDogStand"]],
    "checkver": "github",
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "${LATER}/${FILE}"
            }
        }
    }
}
EOF2
