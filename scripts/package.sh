#!/usr/bin/env bash
# package.sh: packs what players download: the program for one platform, or one definition.
#
#   scripts/package.sh BINARY PLATFORM [OUT]
#   scripts/package.sh --definition FOLDER [OUT]
#
# The first packs zx-sidekick-<version>-<PLATFORM> into OUT (target/package by default): the
# binary, games/ with each definition's tests/ left out (they're its author's, not the
# player's), PLAYER-GUIDE.md and AUTHORS-GUIDE.md (from docs/), README.md, LICENSE,
# THIRD-PARTY.md, and licenses/ with the embedded font's and RustZX's licences and the notices of
# every crate built into the program and of SDL3 (scripts/notices.sh, which needs cargo-about).
# <version> is PACKAGE_VERSION, or the short commit. A .zip for a PLATFORM starting with windows,
# a .tar.gz otherwise. PACKAGE_GAMES packs another games folder.
#
# The second packs one definition's folder as <folder name>.zip in OUT, its files at the top and
# its tests/ left out: the layout docs/definition-format.md gives.
#
# Either refuses, naming it, a tests/ folder or a tape, snapshot or ROM file in what it would
# pack (CLAUDE.md, "Working with copyrighted game code").
set -euo pipefail
cd "$(dirname "$0")/.."

# Copies the files under FROM to TO, leaving out every tests/ folder.
copy_without_tests() {
  local from=$1 to=$2 file
  (cd "$from" && find . -type f -not -path '*/tests/*') | while read -r file; do
    mkdir -p "$to/$(dirname "$file")"
    cp "$from/$file" "$to/$file"
  done
}

refuse_game_files() {
  local refused
  refused=$(cd "$1" && find . \( -type d -name tests \) -o \( -type f \( -iname '*.tap' \
    -o -iname '*.tzx' -o -iname '*.z80' -o -iname '*.sna' -o -iname '*.szx' -o -iname '*.rom' \) \))
  if [ -n "$refused" ]; then
    echo "Not packed: these never go to players:" >&2
    echo "$refused" >&2
    exit 1
  fi
}

# Zips what's in FOLDER, at the zip's top, into ARCHIVE, an absolute path.
zip_contents() {
  local folder=$1 archive=$2
  rm -f "$archive"
  (
    cd "$folder"
    set -- *
    if command -v 7z >/dev/null; then
      7z a -tzip -bd -bso0 "$archive" "$@"
    else
      zip -qrX "$archive" "$@"
    fi
  )
}

if [ "${1:-}" = --definition ]; then
  folder=${2:?the definition folder}
  out=${3:-target/package}
  kind=definition
else
  binary=${1:?the binary to pack}
  platform=${2:?the platform, such as linux, macos-universal or windows-x64}
  out=${3:-target/package}
  kind=program
fi

mkdir -p "$out"
out=$(cd "$out" && pwd)
scratch=$(mktemp -d "$out/stage.XXXXXX")
trap 'rm -rf -- "$scratch"' EXIT

if [ "$kind" = definition ]; then
  copy_without_tests "$folder" "$scratch"
  refuse_game_files "$scratch"
  archive="$out/$(basename "$folder").zip"
  zip_contents "$scratch" "$archive"
  echo "$archive"
  exit 0
fi

name="zx-sidekick-${PACKAGE_VERSION:-$(git rev-parse --short HEAD)}-$platform"
stage="$scratch/$name"
mkdir -p "$stage/games" "$stage/licenses"
cp "$binary" README.md LICENSE THIRD-PARTY.md "$stage/"
cp docs/player-guide.md "$stage/PLAYER-GUIDE.md"
cp docs/definition-format.md "$stage/AUTHORS-GUIDE.md"
cp crates/sidekick-ui/fonts/LICENSE.txt "$stage/licenses/Inter-LICENSE.txt"
cp licenses/RustZX-LICENSE.md "$stage/licenses/"
scripts/notices.sh "$stage/licenses/THIRD-PARTY-NOTICES.txt"
copy_without_tests "${PACKAGE_GAMES:-games}" "$stage/games"
refuse_game_files "$stage"

case "$platform" in
  windows*)
    archive="$out/$name.zip"
    zip_contents "$scratch" "$archive"
    ;;
  *)
    archive="$out/$name.tar.gz"
    tar -czf "$archive" -C "$scratch" "$name"
    ;;
esac
echo "$archive"
