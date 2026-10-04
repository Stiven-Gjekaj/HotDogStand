#!/usr/bin/env bash
# Write the winget manifest of one release, from the SHA256SUMS of that
# release.
#
# The manifest installs the Windows package of the release as a portable
# program in a zip file. A person opens the pull request to winget-pkgs: no
# bot does.
#
# Usage: scripts/winget-manifest.sh <version> <SHA256SUMS> <directory>
#
# The three files go into <directory>/manifests/s/Stiven-Gjekaj/HotDogStand/
# <version>/, which is where winget-pkgs keeps them.

set -euo pipefail

USAGE="usage: winget-manifest.sh <version> <SHA256SUMS> <directory>"
VERSION="${1:?$USAGE}"
SUMS="${2:?$USAGE}"
OUT="${3:?$USAGE}"

ID="Stiven-Gjekaj.HotDogStand"
SCHEMA="1.12.0"
FILE="HotDogStand-windows-x86_64.zip"

# winget-pkgs writes the digest in capitals.
SHA="$(awk -v f="$FILE" '$2 == f { print $1 }' "$SUMS")"
if [[ ! "$SHA" =~ ^[0-9a-f]{64}$ ]]; then
    echo "winget-manifest.sh: $SUMS holds no digest for $FILE" >&2
    exit 1
fi
SHA="$(echo "$SHA" | tr 'a-f' 'A-F')"

DIR="${OUT}/manifests/s/Stiven-Gjekaj/HotDogStand/${VERSION}"
mkdir -p "$DIR"

cat > "${DIR}/${ID}.yaml" <<EOF2
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.version.${SCHEMA}.schema.json
PackageIdentifier: ${ID}
PackageVersion: ${VERSION}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: ${SCHEMA}
EOF2

cat > "${DIR}/${ID}.installer.yaml" <<EOF2
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.installer.${SCHEMA}.schema.json
PackageIdentifier: ${ID}
PackageVersion: ${VERSION}
InstallerType: zip
NestedInstallerType: portable
NestedInstallerFiles:
- RelativeFilePath: HotDogStand\\HotDogStand.exe
  PortableCommandAlias: hotdogstand
Installers:
- Architecture: x64
  InstallerUrl: https://github.com/Stiven-Gjekaj/HotDogStand/releases/download/v${VERSION}/${FILE}
  InstallerSha256: ${SHA}
ManifestType: installer
ManifestVersion: ${SCHEMA}
EOF2

cat > "${DIR}/${ID}.locale.en-US.yaml" <<EOF2
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.defaultLocale.${SCHEMA}.schema.json
PackageIdentifier: ${ID}
PackageVersion: ${VERSION}
PackageLocale: en-US
Publisher: Stiven Gjekaj
PublisherUrl: https://github.com/Stiven-Gjekaj
PackageName: HotDogStand
PackageUrl: https://github.com/Stiven-Gjekaj/HotDogStand
License: MIT
LicenseUrl: https://github.com/Stiven-Gjekaj/HotDogStand/blob/main/LICENSE
ShortDescription: A ticket manager on your own computer, with the look of Windows 7
Description: HotDogStand keeps the tickets of a workspace in one SQLite file on your computer. It shows them in windows with the look of Windows 7, and exports them to JSON or CSV. It has no server and no account.
ReleaseNotesUrl: https://github.com/Stiven-Gjekaj/HotDogStand/releases/tag/v${VERSION}
Tags:
- issue-tracker
- tickets
- todo
- windows-7
ManifestType: defaultLocale
ManifestVersion: ${SCHEMA}
EOF2

echo "$DIR"
