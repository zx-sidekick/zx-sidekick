#!/usr/bin/env bash
# sdl-notices.sh: writes the sdl3-src pins at the end of about.toml, from SDL3's source as cargo
# fetched it, so licenses/THIRD-PARTY-NOTICES.txt carries every notice in it.
#
#   scripts/sdl-notices.sh            rewrites the pins, after the marker line in about.toml
#   scripts/sdl-notices.sh --check    fails, saying so, when the committed pins are out of date
#   scripts/sdl-notices.sh --lines    prints the copyright lines the notices must carry
#
# Pinned: the crate's licence, SDL3's, HIDAPI's BSD licence (it offers three; that one is taken)
# and yuv2rgb's, whole; then every other notice in the source of a file built for a platform the
# program ships on, from its first copyright line to the end of the comment (or the Wayland
# protocol's <copyright>), trailing blank or decoration lines left off. A `//` notice runs as far
# as the `//` lines do. A notice in several files is pinned once, from the first file whose text
# cargo-about can find it in (its `start` first in the file, its `end` first after that).
#
# Left out: the platforms the program doesn't ship on, SDL's tests and build scripts, the two
# HIDAPI licences not taken, OpenVR (off by default), and SDL_init.h, whose only such line is an
# example in its documentation. SDL's own files carry Sam Lantinga's notice, which is SDL3's
# licence, pinned whole.
set -euo pipefail
cd "$(dirname "$0")/.."

marker='# Written by scripts/sdl-notices.sh from here to the end: run it rather than editing.'
copyright='[Cc]opyright[[:space:]]+(\([Cc]\)|©|[0-9])'
left_out='^(src/(test|hidapi/(android|ios|netbsd|hidtest|testgui|m4)|video/(haiku|qnx|openvr))/|src/hidapi/LICENSE-(gpl3|orig)\.txt:|include/SDL3/SDL_init\.h:)'
whole=(LICENSE.md SDL/LICENSE.txt SDL/src/hidapi/LICENSE-bsd.txt SDL/src/video/yuv2rgb/LICENSE)
export LC_ALL=C

sdl_source() {
  cargo metadata --locked --format-version 1 \
    | jq -r '.packages[] | select(.name == "sdl3-src") | .manifest_path' | xargs dirname
}

# path:line for each copyright line that isn't SDL's own, in the files built into the program.
copyright_lines() {
  (cd "$1/SDL" && grep -rIE "$copyright" src include wayland-protocols) \
    | grep -vE "$left_out" | grep -v 'Sam Lantinga' || true
}

sha256() {
  if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1
}

toml_string() {
  local s=${1//\\/\\\\}
  printf '"%s"' "${s//\"/\\\"}"
}

# The pins, as about.toml's last section.
pins() {
  local src=$1 blocks file files path license start end licenses=() n
  blocks=$(mktemp -d)
  trap 'rm -rf -- "$blocks"' RETURN
  copyright_lines "$src" | cut -d: -f1 | sort -u \
    | grep -vxE 'src/hidapi/LICENSE-bsd\.txt|src/video/yuv2rgb/LICENSE' \
    | sed "s|^|$src/SDL/|" >"$blocks/files"
  files=()
  while read -r file; do files+=("$file"); done <"$blocks/files"
  awk -v out="$blocks" -v root="$src/SDL/" -f scripts/sdl-notices.awk "${files[@]}" >"$blocks/index"
  if grep -q $'\037UNKNOWN\037' "$blocks/index"; then
    echo "sdl-notices: a notice under a licence this script doesn't know; add it to label():" >&2
    grep $'\037UNKNOWN\037' "$blocks/index" | cut -d$'\037' -f2 >&2
    return 1
  fi
  licenses=(Zlib BSD-3-Clause)
  while IFS=$'\037' read -r n path license start end; do
    [[ " ${licenses[*]} " == *" $license "* ]] || licenses+=("$license")
  done <"$blocks/index"
  echo "$marker"
  echo '[sdl3-src.clarify]'
  printf 'license = "%s"\n' "$(printf '%s AND ' "${licenses[@]}" | sed 's/ AND $//')"
  for file in "${whole[@]}"; do
    case "$file" in
      *LICENSE-bsd.txt | */yuv2rgb/LICENSE) license=BSD-3-Clause ;;
      *) license=Zlib ;;
    esac
    printf '\n[[sdl3-src.clarify.files]]\npath = "%s"\nlicense = "%s"\nchecksum = "%s"\n' \
      "$file" "$license" "$(sha256 "$src/$file")"
  done
  while IFS=$'\037' read -r n path license start end; do
    printf '\n[[sdl3-src.clarify.files]]\npath = "%s"\nlicense = "%s"\nstart = %s\nend = %s\nchecksum = "%s"\n' \
      "SDL/$path" "$license" "$(toml_string "$start")" "$(toml_string "$end")" "$(sha256 "$blocks/$n.txt")"
  done <"$blocks/index"
}

src=$(sdl_source)
case "${1:-}" in
  --lines)
    copyright_lines "$src" | sed -E 's/^[^:]*://; s/^[[:space:]*#/]*//; s/[[:space:]]+$//' | sort -u
    ;;
  --check | '')
    if ! grep -qxF "$marker" about.toml; then
      echo "sdl-notices: about.toml lacks the line the pins follow:" >&2
      echo "$marker" >&2
      exit 1
    fi
    new=$(mktemp)
    trap 'rm -f -- "$new"' EXIT
    { awk -v marker="$marker" '$0 == marker { exit } { print }' about.toml; pins "$src"; } >"$new"
    if [ "${1:-}" = --check ]; then
      if ! cmp -s "$new" about.toml; then
        echo "sdl-notices: about.toml's sdl3-src pins don't match SDL3's source: run scripts/sdl-notices.sh" >&2
        diff about.toml "$new" >&2 || true
        exit 1
      fi
    else
      cp "$new" about.toml
    fi
    ;;
  *) echo "usage: scripts/sdl-notices.sh [--check | --lines]" >&2; exit 2 ;;
esac
