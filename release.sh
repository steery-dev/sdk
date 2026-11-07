#!/usr/bin/env bash
# Releases the SDK from the checkout it runs in: tags its commit v<version>, the version
# Cargo.toml names, pushes the tag, and records the release on Steery's sdk-releases train.
#
#   ./release.sh
#
# Needs a clean checkout of a commit on origin/main whose Cargo.toml names a new version, and
# `steery` on PATH (or STEERY=/path/to/steery) signed in to the steery-dev workspace.
set -euo pipefail

[[ $# -eq 0 ]] || { echo "usage: $0" >&2; exit 2; }
cd "$(git rev-parse --show-toplevel)"
steery=${STEERY:-steery}

tag=v$(cargo pkgid | sed 's/.*[#@]//')
commit=$(git rev-parse HEAD)
git fetch --quiet --tags origin
git merge-base --is-ancestor "$commit" origin/main || { echo "release.sh: $commit is not on origin/main" >&2; exit 1; }
[[ -z $(git status --porcelain --untracked-files=no) ]] || { echo "release.sh: commit or drop the changes first" >&2; exit 1; }
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  echo "release.sh: $tag exists: raise the version in Cargo.toml first" >&2
  exit 1
fi

# The train records `build` as this script starts, then `shipped`, or `failed` if it stops first.
record() {
  "$steery" --workspace steery-dev rows act --address resource/sdk-releases \
    --action "$1" --params "$2" "${@:3}" >/dev/null
}
release="{\"version\":\"$tag\",\"commit\":\"https://github.com/steery-dev/sdk/commit/$commit\"}"
record build "$release"
step=tagging tagged=0 shipped=0
# A tag that never reached origin is deleted, so the same version can be released again.
trap 'code=$?; if [[ $code -ne 0 && $shipped -eq 0 ]]; then
  [[ $tagged -eq 0 ]] || git tag -d "$tag" >/dev/null || true
  record failed "{\"version\":\"$tag\",\"reason\":\"release.sh exited $code while $step\"}" || true
fi' EXIT

git tag -a "$tag" -m "steery-sdk ${tag#v}" "$commit"
tagged=1
step="pushing the tag"
git push --quiet origin "refs/tags/$tag"
shipped=1
record shipped "$release" --id "shipped-$tag" \
  --words "Pushed [its tag](https://github.com/steery-dev/sdk/tree/$tag)" || {
  echo "release.sh: $tag is pushed, but sdk-releases did not record it. Record it with:" >&2
  echo "  steery --workspace steery-dev rows act --address resource/sdk-releases --action shipped --id shipped-$tag --params '$release'" >&2
  exit 1
}
echo "release.sh: released and recorded $tag"
