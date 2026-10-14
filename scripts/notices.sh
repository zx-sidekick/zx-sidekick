#!/usr/bin/env bash
# notices.sh: writes the third-party notices packed in each release as
# licenses/THIRD-PARTY-NOTICES.txt: the licence of every crate built into the program, on each
# platform it ships for, and SDL3's. Made by cargo-about, from about.toml and scripts/notices.hbs.
#
#   scripts/notices.sh FILE
set -euo pipefail

file=${1:?the file to write}
file="$(cd "$(dirname "$file")" && pwd)/$(basename "$file")"
cd "$(dirname "$0")/.."
if ! command -v cargo-about >/dev/null; then
  echo "notices: cargo-about writes the notices; install it with: cargo install --locked cargo-about" >&2
  exit 1
fi
# Offline, cargo-about reads each crate's licence files from the source cargo fetched, as
# published, rather than from the crate's repository.
cargo fetch --locked --quiet
log=$(mktemp)
trap 'rm -f -- "$log"' EXIT
cargo about generate --locked --offline --fail -c about.toml \
  -m crates/sidekick/Cargo.toml -o "$file" scripts/notices.hbs 2>"$log"
cat "$log" >&2
# cargo-about only warns when a pinned file's checksum no longer matches, and falls back to
# reading every file in the crate: refuse that, so a changed licence is read before it ships.
if grep -qE 'WARN|ERROR' "$log"; then
  echo "notices: cargo-about warned; see above" >&2
  exit 1
fi
