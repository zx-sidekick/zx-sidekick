#!/usr/bin/env bash
# check.sh: the gate, before every PR and in CI.
#
#   scripts/check.sh                  everything
#   scripts/check.sh guard fmt ...    only the named steps (CI runs them in two jobs)
#
# Steps: guard (no tape, snapshot or ROM file committed; CLAUDE.md), package (the packing
# script leaves out tests/ and refuses game files), notes (the release notes), fmt, clippy, test.
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
# The crates cargo builds into the program on this machine, other than the workspace's own, as
# `name version` lines. From cargo tree: cargo metadata's resolve also holds optional crates
# named only by a weak feature (`crate?/feature`), such as SDL3's image, mixer and ttf, which
# aren't built.
built_crates() {
  cargo tree --locked -p sidekick -e normal,build --prefix none -f '{p}' | sed 's/ (\*)$//' \
    | grep -v ' (/' | awk '{ sub(/^v/, "", $2); print $1, $2 }' | LC_ALL=C sort -u
}

# Packs a stand-in game folder with a stand-in binary: the archive is named by its version and
# holds the guides and licences, the notices name every crate built into the program and carry
# SDL3's licence and every other notice in its source (with about.toml's pins up to date), a
# definition's tests/ stays out, and a tape in it is refused. Then packs the stand-in definition
# alone, its files at the zip's top, without its tests/. A definition's other folders (its
# clips/) are packed in both.
package() {
  local scratch archive listing file missing line
  scripts/sdl-notices.sh --check
  scratch=$(mktemp -d)
  trap 'rm -rf -- "$scratch"' RETURN
  mkdir -p "$scratch/games/stand-in/tests" "$scratch/out"
  echo 'schema = 1' >"$scratch/games/stand-in/game.toml"
  echo 'lives = 9' >"$scratch/games/stand-in/tests/start.toml"
  mkdir -p "$scratch/games/stand-in/clips"
  echo 'lives = 9' >"$scratch/games/stand-in/clips/jump.toml"
  echo 'binary' >"$scratch/sidekick"
  archive=$(PACKAGE_GAMES="$scratch/games" PACKAGE_VERSION=9.8.7 \
    scripts/package.sh "$scratch/sidekick" linux-x64 "$scratch/out")
  listing=$(tar -tzf "$archive")
  for file in sidekick games/stand-in/game.toml games/stand-in/clips/jump.toml PLAYER-GUIDE.md \
    AUTHORS-GUIDE.md README.md LICENSE THIRD-PARTY.md licenses/Inter-LICENSE.txt \
    licenses/RustZX-LICENSE.md licenses/THIRD-PARTY-NOTICES.txt; do
    if ! grep -qx "zx-sidekick-9.8.7-linux-x64/$file" <<<"$listing"; then
      echo "package: zx-sidekick-9.8.7-linux-x64.tar.gz should hold $file:" >&2
      echo "$listing" >&2
      return 1
    fi
  done
  tar -xzOf "$archive" zx-sidekick-9.8.7-linux-x64/licenses/THIRD-PARTY-NOTICES.txt >"$scratch/notices.txt"
  sed -n 's/^- //p' "$scratch/notices.txt" | LC_ALL=C sort -u >"$scratch/named.txt"
  built_crates >"$scratch/built.txt"
  missing=$(LC_ALL=C comm -23 "$scratch/built.txt" "$scratch/named.txt")
  if [ -n "$missing" ]; then
    echo "package: THIRD-PARTY-NOTICES.txt should name these crates, built into the program:" >&2
    echo "$missing" >&2
    return 1
  fi
  if ! grep -q '^Copyright (C) .* Sam Lantinga' "$scratch/notices.txt"; then
    echo "package: THIRD-PARTY-NOTICES.txt should carry SDL3's licence" >&2
    return 1
  fi
  scripts/sdl-notices.sh --lines >"$scratch/sdl.txt"
  missing=$(while read -r line; do
    grep -qF -- "$line" "$scratch/notices.txt" || echo "$line"
  done <"$scratch/sdl.txt")
  if [ ! -s "$scratch/sdl.txt" ] || [ -n "$missing" ]; then
    echo "package: THIRD-PARTY-NOTICES.txt should carry these notices from SDL3's source (run scripts/sdl-notices.sh):" >&2
    echo "$missing" >&2
    return 1
  fi
  if grep -q '/tests/' <<<"$listing"; then
    echo "package: the archive should leave out the definition's tests/:" >&2
    echo "$listing" >&2
    return 1
  fi
  archive=$(scripts/package.sh --definition "$scratch/games/stand-in" "$scratch/out")
  listing=$(unzip -Z1 "$archive")
  if [ "$(basename "$archive")" != stand-in.zip ] ||
    [ "$(grep -v '/$' <<<"$listing" | sort | tr '\n' ' ')" != "clips/jump.toml game.toml " ]; then
    echo "package: $(basename "$archive") should hold game.toml and clips/, at its top:" >&2
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

# The release notes on a stand-in history: a release lists the titles since the previous tag,
# counts Dependabot's, names each definition's zip, and carries the notice.
notes() {
  local scratch notes
  scratch=$(mktemp -d)
  trap 'rm -rf -- "$scratch"' RETURN
  stand_in() { git -C "$scratch" -c user.name="${AUTHOR:-Maintainer}" -c user.email=m@example.com \
    -c commit.gpgsign=false -c tag.gpgsign=false "$@"; }
  git init -q "$scratch"
  mkdir -p "$scratch/games/stand-in"
  echo 'title = "Stand-in"' >"$scratch/games/stand-in/game.toml"
  stand_in commit -q --allow-empty -m 'First (#1)'
  stand_in tag v0.1.0
  stand_in commit -q --allow-empty -m 'Second (#2)'
  AUTHOR='dependabot[bot]' stand_in commit -q --allow-empty -m 'Bump a crate (#3)'
  stand_in tag v0.2.0
  notes=$(RELEASE_NOTES_REPO="$scratch" scripts/release-notes.sh v0.2.0)
  if ! grep -qx '## Changes since v0.1.0' <<<"$notes" || ! grep -qx -- '- Second (#2)' <<<"$notes" \
    || grep -q 'First\|Bump' <<<"$notes" || ! grep -q 'One dependency update' <<<"$notes" \
    || ! grep -q 'you need your own copy' <<<"$notes" || ! grep -q 'not affiliated' <<<"$notes" \
    || ! grep -q 'zx-sidekick-0.2.0-windows-x64.zip' <<<"$notes" \
    || ! grep -q "stand-in.zip\` | Stand-in's definition" <<<"$notes"; then
    echo "notes: v0.2.0's notes should list #2 alone, count #3 and carry the notice:" >&2
    echo "$notes" >&2
    return 1
  fi
  notes=$(RELEASE_NOTES_REPO="$scratch" scripts/release-notes.sh v0.3.0)
  if ! grep -qx '## Changes since v0.2.0' <<<"$notes" || grep -q '^- ' <<<"$notes"; then
    echo "notes: a tag not made yet should be previewed from HEAD, since the last tag:" >&2
    echo "$notes" >&2
    return 1
  fi
}

fmt() { cargo fmt --check; }
clippy() { cargo clippy --all-targets -- -D warnings; }
test() { cargo test; }

steps=("$@")
[ ${#steps[@]} -eq 0 ] && steps=(guard package notes fmt clippy test)
for step in "${steps[@]}"; do
  case "$step" in
    guard | package | notes | fmt | clippy | test) echo "== $step"; "$step" ;;
    *) echo "unknown step: $step" >&2; exit 2 ;;
  esac
done
