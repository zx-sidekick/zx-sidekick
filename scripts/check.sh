#!/usr/bin/env bash
# check.sh — the pre-PR gate. Exits non-zero if anything fails.
#
# The processor conformance test needs the Fuse corpus, in assets/ or in
# SK_ASSETS. Without it the gate fails (#78): a pass must mean the processor
# was checked in our bus. SK_NO_FUSE=1 lets it pass anyway, and says so.
#
# Every check that did not run is named again at the end, next to the result.
#
# Gate on the EXIT CODE. Never grep the output.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
failed=()
skipped=()
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
  skipped+=("the dependency policy: cargo-deny is not installed")
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
  skipped+=("THIRD-PARTY.md: cargo-about is not installed")
fi

echo "=== Z80 conformance"
corpus=""
for dir in assets ${SK_ASSETS:+"$SK_ASSETS"}; do
  if [ -f "$dir/tests.in" ] && [ -f "$dir/tests.expected" ]; then
    # Absolute, since cargo runs the test from its own crate's folder.
    corpus=$(cd "$dir" && pwd)
    break
  fi
done
if [ -n "$corpus" ]; then
  echo "the Fuse corpus from $corpus/"
  out=$(FUSE_TESTS="$corpus" cargo test -q -p zx-spectrum --test fuse --locked -- --nocapture 2>&1)
  if ! printf '%s\n' "$out" | grep -q 'Z80 corpus: 1329/1335 cases match exactly, 6 more differ only in the undocumented bits 3 and 5 of F' \
    || ! printf '%s\n' "$out" | grep -q 'Z80 bus activity: 1335/1335 cases match'; then
    failed+=("Z80 conformance")
  fi
elif [ "${SK_NO_FUSE:-}" = 1 ]; then
  skipped+=("Z80 conformance: SK_NO_FUSE=1")
else
  failed+=("Z80 conformance: no Fuse corpus in assets/ or SK_ASSETS (see assets/README.md; SK_NO_FUSE=1 skips it)")
fi

# The local checks against the player's copy of the game, which CI cannot
# run: they need the tape and a Spectrum ROM, and those are never committed.
# Give their location with SK_ASSETS (a folder holding starquake.tap and
# 48.rom).
if [ -n "${SK_ASSETS:-}" ] && [ -f "$SK_ASSETS/starquake.tap" ]; then
  run "keys (joystick in every control method)" cargo run -q --release -p sk-check --locked -- keys "$SK_ASSETS"
  run "facts (the panel's entry points vs the game)" cargo run -q --release -p sk-check --locked -- facts "$SK_ASSETS"
  run "training (the switches vs the game)" cargo run -q --release -p sk-check --locked -- training "$SK_ASSETS"
  run "map (exits and walls vs walks)" cargo run -q --release -p sk-check --locked -- map "$SK_ASSETS" 60
  if [ -f "$SK_ASSETS/48.rom" ]; then
    run "entry (real ROM loader)"  cargo run -q --release -p sk-check --locked -- entry "$SK_ASSETS"
    run "rom (answers vs real ROM)" cargo run -q --release -p sk-check --locked -- rom "$SK_ASSETS" 6000
  else
    skipped+=("entry and rom: no 48.rom in SK_ASSETS")
  fi
else
  skipped+=("the checks against the game: SK_ASSETS is not set, or has no starquake.tap")
fi

if [ ${#skipped[@]} -ne 0 ]; then
  printf 'NOT RUN: %s\n' "${skipped[@]}"
fi
if [ ${#failed[@]} -ne 0 ]; then
  printf 'FAILED: %s\n' "${failed[@]}"
  exit 1
fi
if [ ${#skipped[@]} -ne 0 ]; then
  echo "every check that ran passed"
else
  echo "all checks passed"
fi
