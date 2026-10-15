#!/usr/bin/env bash
# release-tag.sh: checks a release tag against the workspace version in Cargo.toml, so a release
# can't go out under one number and report another (`sidekick --version`).
#
#   scripts/release-tag.sh TAG
#
# Prints `release` for v<version>, and `prerelease` for v<version>-rc.N (N a number), a trial run
# of that release. Any other tag fails, naming the version.
set -euo pipefail
cd "$(dirname "$0")/.."

tag=${1:?the release tag, such as v0.1.0}
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
trial=${tag#"v$version-rc."}

if [ "$tag" = "v$version" ]; then
  echo release
elif [ "$trial" != "$tag" ] && [[ $trial =~ ^[0-9]+$ ]]; then
  echo prerelease
else
  echo "::error::The tag is $tag, but Cargo.toml's version is $version: tag v$version (or v$version-rc.1 for a trial run), or move the version first." >&2
  exit 1
fi
