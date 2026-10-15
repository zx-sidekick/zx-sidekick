#!/usr/bin/env bash
# release-notes.sh: prints a release's notes, as Markdown, for the draft release.
#
#   scripts/release-notes.sh TAG
#
# The notice, the downloads (one archive a platform, and each definition in games/ alone), and
# the subjects of the commits on main since the previous v* tag: each a merged pull request's
# title, as squash-merging leaves it. Dependabot's are counted, not listed. The first release,
# with no v* tag before it, has a summary for players instead, docs/release-notes/<version>.md.
# A trial run's tag, v<version>-rc.N, has the notes its release would have, its own number on the
# downloads; it is never the previous tag. A TAG not made yet is previewed from HEAD.
# RELEASE_NOTES_REPO reads another repository's history, games/ and docs/.
set -euo pipefail

repo=${RELEASE_NOTES_REPO:-$(cd "$(dirname "$0")/.." && pwd)}
tag=${1:?the release tag, such as v0.1.0}
version=${tag#v}
release=${version%-rc.*}

if git -C "$repo" rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  end=$tag
  since="$tag^"
else
  end=HEAD
  since=HEAD
fi
previous=$(git -C "$repo" describe --tags --abbrev=0 --match 'v*' --exclude 'v*-rc.*' "$since" \
  2>/dev/null || true)

cat <<EOF
ZX Sidekick $version plays original ZX Spectrum games from your own copy, in an emulated Spectrum, with a gamepad, optional training switches, a guidance panel, save games and kept high scores.

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

if [ -z "$previous" ]; then
  summary="$repo/docs/release-notes/$release.md"
  if [ ! -f "$summary" ]; then
    echo "release-notes: $tag is the first release, whose notes are docs/release-notes/$release.md: write it" >&2
    exit 1
  fi
  cat "$summary"
  exit 0
fi

echo "## Changes since $previous"
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
