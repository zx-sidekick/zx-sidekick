#!/usr/bin/env bash
# package.sh: packs a ready-to-run archive of the program for one platform.
#
#   scripts/package.sh BINARY PLATFORM [OUT]
#
# Packs zx-sidekick-<PLATFORM>-<short commit> into OUT (target/package by default): the
# binary, games/ with each definition's tests/ left out (they're its author's, not the
# player's), README.md, LICENSE and THIRD-PARTY.md. A .zip for windows, a .tar.gz otherwise.
# Refuses, naming it, a tests/ folder or a tape, snapshot or ROM file in what it would pack
# (CLAUDE.md, "Working with copyrighted game code"). PACKAGE_GAMES packs another games folder.
set -euo pipefail
cd "$(dirname "$0")/.."

binary=${1:?the binary to pack}
platform=${2:?the platform: linux, macos or windows}
out=${3:-target/package}
games=${PACKAGE_GAMES:-games}
name="zx-sidekick-$platform-$(git rev-parse --short HEAD)"

mkdir -p "$out"
stage="$(mktemp -d "$out/stage.XXXXXX")/$name"
mkdir -p "$stage/games"
cp "$binary" README.md LICENSE THIRD-PARTY.md "$stage/"
(cd "$games" && find . -type f -not -path './*/tests/*') | while read -r file; do
  mkdir -p "$stage/games/$(dirname "$file")"
  cp "$games/$file" "$stage/games/$file"
done

refused=$(cd "$stage" && find . \( -type d -name tests \) -o \( -type f \( -iname '*.tap' \
  -o -iname '*.tzx' -o -iname '*.z80' -o -iname '*.sna' -o -iname '*.szx' -o -iname '*.rom' \) \))
if [ -n "$refused" ]; then
  echo "Not packed: these never go to players:" >&2
  echo "$refused" >&2
  exit 1
fi

parent=$(dirname "$stage")
if [ "$platform" = windows ]; then
  archive="$out/$name.zip"
  rm -f "$archive"
  if command -v 7z >/dev/null; then
    (cd "$parent" && 7z a -tzip -bd -bso0 "$OLDPWD/$archive" "$name")
  else
    (cd "$parent" && zip -qr "$OLDPWD/$archive" "$name")
  fi
else
  archive="$out/$name.tar.gz"
  tar -czf "$archive" -C "$parent" "$name"
fi
echo "$archive"
