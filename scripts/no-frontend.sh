#!/usr/bin/env bash
# no-frontend.sh — fails if the machine or the checks depend on a window,
# sound, input or file-dialog crate. Those belong to the app alone, so the
# machine stays something any frontend, or none, can run.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 1

frontend='^(winit|pixels|wgpu|cpal|gilrs|rfd|fontdue|zip|windows-sys) '
found=$(cargo tree --locked -e normal --prefix none -p zx-core -p zx-spectrum -p sidekick -p starquake -p sk-check -p sk-lab \
  | grep -E "$frontend" | sort -u || true)
if [ -n "$found" ]; then
  echo "the machine or the checks depend on frontend crates:"
  echo "$found"
  exit 1
fi
echo "the machine and the checks have no frontend dependencies"
