#!/usr/bin/env bash
# check.sh — the pre-PR gate. Exits non-zero if anything fails.
#
# The processor conformance test needs the Fuse corpus in assets/; without
# it that check is reported as skipped, loudly.
#
# Gate on the EXIT CODE. Never grep the output.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
failed=()
run() {
  local name="$1"; shift
  echo "=== $name"
  if "$@"; then return 0; fi
  failed+=("$name")
}

run "fmt"     cargo fmt --all --check
run "build"   cargo build --workspace --all-targets --locked
run "test"    cargo test --workspace --locked
run "clippy"  cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" run "doc" cargo doc --workspace --no-deps --locked
# The machine and the checks are meant to have no platform dependencies at
# all: windows, sound, input and file dialogs belong to the app alone.
run "no-frontend" scripts/no-frontend.sh
# What the dependency tree is allowed to contain. Skipped rather than failed
# when the tool is absent, since it is an install away:
# `cargo install cargo-deny --locked`.
if command -v cargo-deny > /dev/null; then
  run "cargo-deny" cargo deny check
else
  echo "!!! cargo-deny not installed; the dependency policy was NOT checked."
fi
# The attributions shipped with a binary. Same story:
# `cargo install cargo-about --locked --features cli`.
if command -v cargo-about > /dev/null; then
  echo "=== third-party attributions"
  cargo about generate --all-features about.hbs -o "${TMPDIR:-/tmp}/THIRD-PARTY.md" 2> /dev/null
  if diff -q THIRD-PARTY.md "${TMPDIR:-/tmp}/THIRD-PARTY.md" > /dev/null; then
    echo "THIRD-PARTY.md is current"
  else
    failed+=("THIRD-PARTY.md is stale: cargo about generate --all-features about.hbs -o THIRD-PARTY.md")
  fi
else
  echo "!!! cargo-about not installed; THIRD-PARTY.md was NOT checked."
fi

echo "=== Z80 conformance"
if [ -f assets/tests.in ] && [ -f assets/tests.expected ]; then
  out=$(cargo test -q -p zx-spectrum --test fuse --locked -- --nocapture 2>&1)
  if ! printf '%s\n' "$out" | grep -q 'Z80 corpus: 1329/1335 cases match exactly, 6 more differ only in the undocumented bits 3 and 5 of F' \
    || ! printf '%s\n' "$out" | grep -q 'Z80 bus activity: 1335/1335 cases match'; then
    failed+=("Z80 conformance")
  fi
else
  echo "!!! the Fuse corpus is not in assets/; conformance was NOT checked."
fi

# The local checks against the player's copy of the game, which CI cannot
# run: they need the tape and a Spectrum ROM, and those are never committed.
# Give their location with SK_ASSETS (a folder holding starquake.tap and
# 48.rom).
if [ -n "${SK_ASSETS:-}" ] && [ -f "$SK_ASSETS/starquake.tap" ]; then
  run "keys (joystick in every control method)" cargo run -q --release -p sk-check --locked -- keys "$SK_ASSETS"
  run "facts (the panel's entry points vs the game)" cargo run -q --release -p sk-check --locked -- facts "$SK_ASSETS"
  if [ -f "$SK_ASSETS/48.rom" ]; then
    run "entry (real ROM loader)"  cargo run -q --release -p sk-check --locked -- entry "$SK_ASSETS"
    run "rom (answers vs real ROM)" cargo run -q --release -p sk-check --locked -- rom "$SK_ASSETS" 6000
  else
    echo "!!! no 48.rom in SK_ASSETS; the ROM checks did NOT run."
  fi
else
  echo "!!! SK_ASSETS not set (or no starquake.tap in it); the checks against the game did NOT run."
fi

if [ ${#failed[@]} -ne 0 ]; then
  printf 'FAILED: %s\n' "${failed[@]}"
  exit 1
fi
echo "all checks passed"
