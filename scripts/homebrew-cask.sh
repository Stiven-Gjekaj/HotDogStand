#!/usr/bin/env bash
# Write the Homebrew cask of one release, from the SHA256SUMS of that
# release.
#
# The cask installs HotDogStand.app, which runs on Apple silicon and on Intel.
# A person puts it into the tap and commits it there: no bot pushes to the
# tap.
#
# Usage: scripts/homebrew-cask.sh <version> <SHA256SUMS> > ../homebrew-tap/Casks/hotdogstand.rb

set -euo pipefail

USAGE="usage: homebrew-cask.sh <version> <SHA256SUMS>"
VERSION="${1:?$USAGE}"
SUMS="${2:?$USAGE}"

FILE="HotDogStand-macos-universal.zip"
SHA="$(awk -v f="$FILE" '$2 == f { print $1 }' "$SUMS")"
if [[ ! "$SHA" =~ ^[0-9a-f]{64}$ ]]; then
    echo "homebrew-cask.sh: $SUMS holds no digest for $FILE" >&2
    exit 1
fi

cat <<CASK
# scripts/homebrew-cask.sh in the HotDogStand repository writes this file
# from the SHA256SUMS of the release. Change the script, and not this file.
cask "hotdogstand" do
  version "${VERSION}"
  sha256 "${SHA}"

  url "https://github.com/Stiven-Gjekaj/HotDogStand/releases/download/v#{version}/${FILE}"
  name "HotDogStand"
  desc "Ticket manager on your own computer, with the look of Windows 7"
  homepage "https://github.com/Stiven-Gjekaj/HotDogStand"

  depends_on macos: :big_sur

  app "HotDogStand.app"

  zap trash: "~/Library/Application Support/HotDogStand"

  caveats <<~EOS
    No paid certificate signed HotDogStand, so macOS can refuse to open it.
    Install it with --no-quarantine, or take the mark off after the install:

      xattr -dr com.apple.quarantine #{appdir}/HotDogStand.app

    HotDogStand keeps your tickets in
    ~/Library/Application Support/HotDogStand/workspace.db.
    An uninstall does not remove them. "brew uninstall --zap" does.
  EOS
end
CASK
