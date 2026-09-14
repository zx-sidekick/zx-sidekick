#!/usr/bin/env bash
# check.sh — the gate. Exits non-zero if anything fails.
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

if [ ${#failed[@]} -ne 0 ]; then
  printf 'FAILED: %s\n' "${failed[@]}"
  exit 1
fi
echo "all checks passed"
