#!/usr/bin/env bash
# Each game's changelog, from the commit messages (#124, #132). Every commit
# on main is one pull request, squash-merged, with the PR's title and number
# as its subject, so the subjects are the list of what changed.
#
#   scripts/changelog.sh GAME         writes games/GAME/CHANGELOG.md
#   scripts/changelog.sh --notes TAG  prints TAG's list, for its release notes
#
# A game's releases are tagged GAME-vX.Y.Z and its candidates
# GAME-vX.Y.Z-rc.N; Starquake's releases from before the monorepo are the
# plain vX.Y.Z tags. A release lists everything since the release before
# it; a candidate lists what is new since the tag before it, release or
# candidate; either oldest first. A game's list leaves out what touched only
# other games' folders. The CHANGELOG.md has a section for each release, the
# newest release first: a candidate's changes are in the release it led to.
set -euo pipefail

REPO="https://github.com/zx-sidekick/zx-sidekick"
cd "$(git rev-parse --show-toplevel)"

# The game a tag belongs to, and its version (v0.3.0, v0.3.0-rc.1).
game_of() {
  case "$1" in
    v[0-9]*) echo starquake ;;
    *-v[0-9]*) echo "${1%%-v[0-9]*}" ;;
    *) return 1 ;;
  esac
}
version_of() {
  case "$1" in
    v[0-9]*) echo "$1" ;;
    *) echo "${1#"$(game_of "$1")"-}" ;;
  esac
}

is_release() { [[ "$(version_of "$1")" != *-* ]]; }

# Every tag of $1, oldest first, with a candidate before its release. A tag
# that is not vX.Y.Z or vX.Y.Z-rc.N is no release of it, and is left out.
tags() {
  local game=$1 t
  {
    git tag --list "$game-v*"
    if [ "$game" = starquake ]; then git tag --list 'v*'; fi
  } | while read -r t; do
    if [ "$(game_of "$t" 2> /dev/null)" = "$game" ]; then
      echo "$t $(version_of "$t")"
    fi
  done | awk '
    $2 ~ /^v[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$/ {
      v = substr($2, 2); rc = -1
      i = index(v, "-rc.")
      if (i) { rc = substr(v, i + 4); v = substr(v, 1, i - 1) }
      split(v, n, ".")
      printf "%06d%06d%06d%d%06d %s\n", n[1], n[2], n[3], (rc < 0), (rc < 0 ? 0 : rc), $1
    }' | sort | cut -d' ' -f2
}

# The tag the list for $1 starts after: the release before it for a release,
# the tag before it for a candidate. Empty for the first.
since() {
  local tag=$1 prev="" t all
  # Read first, so a failure stops the script instead of ending the loop.
  all=$(tags "$(game_of "$tag")")
  while read -r t; do
    [ "$t" = "$tag" ] && break
    if ! is_release "$tag" || is_release "$t"; then
      prev=$t
    fi
  done <<< "$all"
  echo "$prev"
}

# The commits in $1's list, oldest first, in the order they were merged, as
# GitHub's own release notes list them, one Markdown line each, with the
# pull request's number linked. Commits that touched only other games'
# folders are not this game's.
list() {
  local tag=$1 game from other
  game=$(game_of "$tag")
  from=$(since "$tag")
  local paths=(.)
  for other in games/*/; do
    other=${other%/}
    [ "$other" = "games/$game" ] || paths+=(":(exclude)$other")
  done
  git log --reverse --format='%s' "${from:+$from..}$tag" -- "${paths[@]}" \
    | sed -E "s|\(#([0-9]+)\)$|([#\1]($REPO/pull/\1))|; s|^|- |"
}

if [ "${1:-}" = "--notes" ]; then
  tag=${2:?"usage: scripts/changelog.sh --notes TAG"}
  git rev-parse -q --verify "refs/tags/$tag" > /dev/null || {
    echo "no tag $tag" >&2
    exit 1
  }
  game_of "$tag" > /dev/null || {
    echo "$tag is no game's tag: GAME-vX.Y.Z, or vX.Y.Z for Starquake" >&2
    exit 1
  }
  list "$tag"
  exit 0
fi

game=${1:?"usage: scripts/changelog.sh GAME, or scripts/changelog.sh --notes TAG"}
[ -d "games/$game" ] || {
  echo "no game in games/$game" >&2
  exit 1
}
newest_first=$(tags "$game" | sed -n '1!G;h;$p')
{
  echo "# Changelog"
  echo
  echo "What changed in each release, from the commit messages: every line is one pull request, squash-merged into \`main\`, in the order they were merged, leaving out what touched only other games. Written by \`scripts/changelog.sh $game\`, which is run when a release is tagged; do not edit it by hand. A release candidate's changes are in the release it led to."
  while read -r tag; do
    is_release "$tag" || continue
    echo
    echo "## $(version_of "$tag") ($(git log -1 --format=%cs "$tag"))"
    echo
    list "$tag"
  done <<< "$newest_first"
} > "games/$game/CHANGELOG.md"
