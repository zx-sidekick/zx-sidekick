#!/usr/bin/env bash
# render-mockups.sh: renders docs/mockups/starquake/mockups.html into its PNGs with headless
# Chrome. The mockups hold nothing of the game's; see the comment at the top of the HTML.
#
#   scripts/render-mockups.sh            every mockup
#   scripts/render-mockups.sh menu ...   only those named
set -euo pipefail
cd "$(dirname "$0")/.."
CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
DIR=docs/mockups/starquake
ALL=(play-level0 play-level1 play-level2 play-level3 play-level4 play-level5 play-level6
  menu paused booth-choose booth-spell saved-note offer game-over tape-prompt tape-prompt-wrong-dump)
names=("$@")
[ ${#names[@]} -eq 0 ] && names=("${ALL[@]}")
for name in "${names[@]}"; do
  case "$name" in tape-prompt*) size=960,768 ;; *) size=1368,768 ;; esac
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 \
    --window-size="$size" --virtual-time-budget=8000 \
    --screenshot="$PWD/$DIR/$name.png" "file://$PWD/$DIR/mockups.html#$name" >/dev/null 2>&1
  echo "$DIR/$name.png"
done
