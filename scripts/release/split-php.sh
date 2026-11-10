#!/bin/sh
# Pushes sdk/php, as a repository of its own, to the repository Packagist
# tracks (Packagist reads composer.json at the root of a repository, and in
# this one it is at sdk/php/).
#
#   scripts/release/split-php.sh <version> <destination> [--dry-run]
#
# <destination> is the URL of the split repository (a path works too). It may
# hold a token, so no message prints it. Run from a full clone (fetch-depth 0)
# with the credentials the push needs in the URL or in git's configuration. The
# split is a function of this repository's history, so a second run makes the
# same commits.
#
# * The branch `main` is pushed without force. If the destination's `main`
#   is not an ancestor of the split, someone pushed there, and this fails.
# * The tag v<version> is made on the split of the commit that the tag
#   v<version> of THIS repository points at, which is the commit checked out:
#   the split repository never holds a version this one has not released. A tag
#   that is already on the destination is left alone when it is at the same
#   commit, and fails when it is at another.
# * With --dry-run nothing is pushed: the split is made and the destination is
#   read. The tag of this repository may be missing then (a dry run comes before
#   the tag), and the split is of the commit checked out.
set -eu

if [ $# -lt 2 ]; then
    echo "usage: $0 <version> <destination> [--dry-run]" >&2
    exit 2
fi
version=$1
destination=$2
dry=false
[ "${3:-}" = "--dry-run" ] && dry=true
tag="v$version"

[ "$(git rev-parse --is-shallow-repository)" = false ] \
    || { echo "split-php: the clone is shallow; the split needs the whole history" >&2; exit 1; }

if git rev-parse --verify --quiet "refs/tags/$tag" >/dev/null; then
    [ "$(git rev-parse "$tag^{commit}")" = "$(git rev-parse HEAD)" ] \
        || { echo "split-php: $tag is not the commit checked out" >&2; exit 1; }
elif [ "$dry" = true ]; then
    echo "split-php: this repository has no tag $tag yet"
else
    echo "split-php: this repository has no tag $tag" >&2
    exit 1
fi

split=$(git subtree split --quiet --prefix=sdk/php HEAD)
echo "split-php: sdk/php at $(git rev-parse --short HEAD) is $split"

remote_tag=$(git ls-remote "$destination" "refs/tags/$tag^{}" "refs/tags/$tag" | tail -n 1 | cut -f1)
if [ -n "$remote_tag" ]; then
    if [ "$remote_tag" != "$split" ]; then
        echo "split-php: the split repository has $tag at $remote_tag, not $split" >&2
        exit 1
    fi
    echo "split-php: the split repository already has $tag at $split"
fi

if [ "$dry" = true ]; then
    echo "split-php: dry run, nothing pushed"
    exit 0
fi

# Without force: a destination `main` that is not an ancestor is refused.
git push --quiet "$destination" "$split:refs/heads/main"
[ -n "$remote_tag" ] || git push --quiet "$destination" "$split:refs/tags/$tag"
echo "split-php: pushed $split as main and $tag"
