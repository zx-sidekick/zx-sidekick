#!/usr/bin/env bash
# check.sh: the gate, before every PR and in CI.
#
#   scripts/check.sh                  everything
#   scripts/check.sh guard fmt ...    only the named steps (CI runs them in two jobs)
#
# Steps: guard (no tape, snapshot or ROM file committed; CLAUDE.md), fmt, clippy, test.
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
fmt() { cargo fmt --check; }
clippy() { cargo clippy --all-targets -- -D warnings; }
test() { cargo test; }

steps=("$@")
[ ${#steps[@]} -eq 0 ] && steps=(guard fmt clippy test)
for step in "${steps[@]}"; do
  case "$step" in
    guard | fmt | clippy | test) echo "== $step"; "$step" ;;
    *) echo "unknown step: $step" >&2; exit 2 ;;
  esac
done
