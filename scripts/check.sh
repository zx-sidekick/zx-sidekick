#!/usr/bin/env bash
# check.sh: the gate, before every PR and in CI.
#
#   scripts/check.sh                  everything
#   scripts/check.sh guard fmt ...    only the named steps (CI runs them in two jobs)
#
# Steps: guard (no tape, snapshot or ROM file committed; CLAUDE.md), package (the packing
# script leaves out tests/ and refuses game files), fmt, clippy, test.
set -euo pipefail
cd "$(dirname "$0")/.."

guard() {
  local found
  found=$(git ls-files | grep -iE '\.(tap|tzx|z80|sna|szx|rom)$' || true)
  if [ -n "$found" ]; then
    echo "These must never be committed (CLAUDE.md, \"Working with copyrighted game code\"):" >&2
    echo "$found" >&2
    return 1
  fi
}
# Packs a stand-in game folder with a stand-in binary: the archive is named by its version and
# holds the guides and licences, a definition's tests/ stays out, and a tape in it is refused.
# Then packs the stand-in definition alone, its files at the zip's top, without its tests/.
package() {
  local scratch archive listing file
  scratch=$(mktemp -d)
  trap 'rm -rf -- "$scratch"' RETURN
  mkdir -p "$scratch/games/stand-in/tests" "$scratch/out"
  echo 'schema = 1' >"$scratch/games/stand-in/game.toml"
  echo 'lives = 9' >"$scratch/games/stand-in/tests/start.toml"
  echo 'binary' >"$scratch/sidekick"
  archive=$(PACKAGE_GAMES="$scratch/games" PACKAGE_VERSION=9.8.7 \
    scripts/package.sh "$scratch/sidekick" linux-x64 "$scratch/out")
  listing=$(tar -tzf "$archive")
  for file in sidekick games/stand-in/game.toml PLAYER-GUIDE.md AUTHORS-GUIDE.md README.md \
    LICENSE THIRD-PARTY.md licenses/Inter-LICENSE.txt licenses/RustZX-LICENSE.md; do
    if ! grep -qx "zx-sidekick-9.8.7-linux-x64/$file" <<<"$listing"; then
      echo "package: zx-sidekick-9.8.7-linux-x64.tar.gz should hold $file:" >&2
      echo "$listing" >&2
      return 1
    fi
  done
  if grep -q '/tests/' <<<"$listing"; then
    echo "package: the archive should leave out the definition's tests/:" >&2
    echo "$listing" >&2
    return 1
  fi
  archive=$(scripts/package.sh --definition "$scratch/games/stand-in" "$scratch/out")
  listing=$(unzip -Z1 "$archive")
  if [ "$(basename "$archive")" != stand-in.zip ] || [ "$listing" != game.toml ]; then
    echo "package: $(basename "$archive") should hold game.toml alone, at its top:" >&2
    echo "$listing" >&2
    return 1
  fi
  echo 'tape' >"$scratch/games/stand-in/game.tap"
  if PACKAGE_GAMES="$scratch/games" scripts/package.sh "$scratch/sidekick" linux "$scratch/out" 2>/dev/null; then
    echo "package: a tape in games/ should be refused" >&2
    return 1
  fi
  if scripts/package.sh --definition "$scratch/games/stand-in" "$scratch/out" 2>/dev/null; then
    echo "package: a tape in a definition should be refused" >&2
    return 1
  fi
}
fmt() { cargo fmt --check; }
clippy() { cargo clippy --all-targets -- -D warnings; }
test() { cargo test; }

steps=("$@")
[ ${#steps[@]} -eq 0 ] && steps=(guard package fmt clippy test)
for step in "${steps[@]}"; do
  case "$step" in
    guard | package | fmt | clippy | test) echo "== $step"; "$step" ;;
    *) echo "unknown step: $step" >&2; exit 2 ;;
  esac
done
