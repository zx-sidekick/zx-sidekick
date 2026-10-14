#!/usr/bin/env bash
# notices.sh: writes the third-party notices packed in each release as
# licenses/THIRD-PARTY-NOTICES.txt: the licence of every crate built into the program, on each
# platform it ships for, and SDL3's. Made by cargo-about, from about.toml and scripts/notices.hbs.
#
#   scripts/notices.sh FILE
set -euo pipefail
cd "$(dirname "$0")/.."

file=${1:?the file to write}
if ! command -v cargo-about >/dev/null; then
  echo "notices: cargo-about writes the notices; install it with: cargo install --locked cargo-about" >&2
  exit 1
fi
# Offline, cargo-about reads each crate's licence files from the source cargo fetched, as
# published, rather than from the crate's repository.
cargo fetch --locked --quiet
cargo about generate --locked --offline --fail -c about.toml \
  -m crates/sidekick/Cargo.toml -o "$file" scripts/notices.hbs
