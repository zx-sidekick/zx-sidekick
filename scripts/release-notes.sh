#!/usr/bin/env bash
# release-notes.sh: prints a release's notes, as Markdown, for the draft release.
#
#   scripts/release-notes.sh TAG
#
# The notice, the downloads (one archive a platform, and each definition in games/ alone), and
# the subjects of the commits on main since the previous v* tag: each a merged pull request's
# title, as squash-merging leaves it. Dependabot's are counted, not listed. A TAG not made yet is
# previewed from HEAD. RELEASE_NOTES_REPO reads another repository's history and games/.
set -euo pipefail

repo=${RELEASE_NOTES_REPO:-$(cd "$(dirname "$0")/.." && pwd)}
tag=${1:?the release tag, such as v0.1.0}
version=${tag#v}

if git -C "$repo" rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  end=$tag
  previous=$(git -C "$repo" describe --tags --abbrev=0 --match 'v*' "$tag^" 2>/dev/null || true)
else
  end=HEAD
  previous=$(git -C "$repo" describe --tags --abbrev=0 --match 'v*' HEAD 2>/dev/null || true)
fi

cat <<EOF
ZX Sidekick $version plays original ZX Spectrum games from your own copy, in an emulated Spectrum, with a gamepad, optional training switches, a guidance panel and save games.

**No game is included; you need your own copy.** ZX Sidekick is not affiliated with or endorsed by any game's rights holders. The Spectrum ROMs are included with Amstrad's permission (\`THIRD-PARTY.md\`).

## Downloads

| File | For |
|---|---|
| \`zx-sidekick-$version-macos-universal.tar.gz\` | macOS 11 or later, Apple silicon and Intel |
| \`zx-sidekick-$version-windows-x64.zip\` | 64-bit Windows 10 or 11 |
| \`zx-sidekick-$version-linux-x64.tar.gz\` | 64-bit Linux on x86 |
EOF
for folder in "$repo"/games/*/; do
  id=$(basename "$folder")
  title=$(sed -n 's/^title = "\(.*\)"$/\1/p' "$folder/game.toml" 2>/dev/null | head -n 1)
  echo "| \`$id.zip\` | ${title:-$id}'s definition alone, for a program already installed |"
done
cat <<EOF

The program isn't signed: \`PLAYER-GUIDE.md\`, in each archive, says how to open it the first time on macOS and on Windows, where to get each game's tape, and how to play.

EOF

if [ -n "$previous" ]; then
  echo "## Changes since $previous"
else
  echo "## Changes"
fi
echo
updates=0
while IFS=$'\t' read -r author subject; do
  if [ "$author" = 'dependabot[bot]' ]; then
    updates=$((updates + 1))
  else
    echo "- $subject"
  fi
done < <(git -C "$repo" log --first-parent --format='%an%x09%s' "${previous:+$previous..}$end")
case $updates in
  0) ;;
  1) echo "- One dependency update, by Dependabot." ;;
  *) echo "- $updates dependency updates, by Dependabot." ;;
esac
