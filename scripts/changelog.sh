#!/usr/bin/env bash
# The changelog, from the commit messages (#124). Every commit on main is one
# pull request, squash-merged, with the PR's title and number as its subject,
# so the subjects are the list of what changed.
#
#   scripts/changelog.sh              writes games/starquake/CHANGELOG.md
#   scripts/changelog.sh --notes TAG  prints TAG's list, for its release notes
#
# A release (vX.Y.Z) lists everything since the release before it; a
# candidate (vX.Y.Z-rc.N) lists what is new since the tag before it, release
# or candidate; either oldest first. games/starquake/CHANGELOG.md has a section for each
# release, the newest release first: a candidate's changes are in the release
# it led to.
set -euo pipefail

REPO="https://github.com/zx-sidekick/zx-sidekick"

# Every version tag, oldest first, with a candidate before its release.
tags() {
  git -c versionsort.suffix=-rc tag --list 'v*' --sort=v:refname
}

is_release() { [[ "$1" != *-* ]]; }

# The tag the list for $1 starts after: the release before it for a release,
# the tag before it for a candidate. Empty for the first.
since() {
  local tag=$1 prev=""
  while read -r t; do
    [ "$t" = "$tag" ] && break
    if ! is_release "$tag" || is_release "$t"; then
      prev=$t
    fi
  done < <(tags)
  echo "$prev"
}

# The commits in $1's list, oldest first, in the order they were merged, as
# GitHub's own release notes list them, one Markdown line each, with the
# pull request's number linked.
list() {
  local tag=$1 from
  from=$(since "$tag")
  git log --reverse --format='%s' "${from:+$from..}$tag" \
    | sed -E "s|\(#([0-9]+)\)$|([#\1]($REPO/pull/\1))|; s|^|- |"
}

if [ "${1:-}" = "--notes" ]; then
  tag=${2:?"usage: scripts/changelog.sh --notes TAG"}
  git rev-parse -q --verify "refs/tags/$tag" > /dev/null || {
    echo "no tag $tag" >&2
    exit 1
  }
  list "$tag"
  exit 0
fi

cd "$(git rev-parse --show-toplevel)"
{
  echo "# Changelog"
  echo
  echo "What changed in each release, from the commit messages: every line is one pull request, squash-merged into \`main\`, in the order they were merged. Written by \`scripts/changelog.sh\`, which is run when a release is tagged; do not edit it by hand. A release candidate's changes are in the release it led to."
  while read -r tag; do
    is_release "$tag" || continue
    echo
    echo "## $tag ($(git log -1 --format=%cs "$tag"))"
    echo
    list "$tag"
  done < <(tags | sed -n '1!G;h;$p')
} > games/starquake/CHANGELOG.md
