#!/bin/sh
# Makes the fixtures of a released version: a workspace file and its JSON
# export, made by the code of that version. The test in
# crates/hds-store/tests/fixtures.rs opens them with the current code.
#
#     scripts/make-fixtures.sh v1.0.0
#
# Run it once for each new release. Do not change a fixture after it is in
# the repository. A fixture shows what a released version wrote.
set -eu

tag=$1
version=${tag#v}
root=$(git rev-parse --show-toplevel)
out="$root/crates/hds-store/tests/fixtures/$version"
work=$(mktemp -d)
trap 'git -C "$root" worktree remove --force "$work/src" 2>/dev/null || true; rm -rf "$work"' EXIT

git -C "$root" worktree add --quiet --detach "$work/src" "$tag"
cp "$root/scripts/fixtures/export.rs" "$work/src/crates/hds-store/examples/fixture_export.rs"
(
    cd "$work/src"
    CARGO_TARGET_DIR="$root/target/fixtures" cargo run --quiet -p hds-store --example demo -- "$work/workspace.hotdogstand"
    CARGO_TARGET_DIR="$root/target/fixtures" cargo run --quiet -p hds-store --example fixture_export -- "$work/workspace.hotdogstand" "$work/export.json"
)
if [ -e "$work/workspace.hotdogstand-wal" ]; then
    echo "make-fixtures: the workspace file was not closed" >&2
    exit 1
fi

mkdir -p "$out"
cp "$work/workspace.hotdogstand" "$work/export.json" "$out/"
echo "made $out"
