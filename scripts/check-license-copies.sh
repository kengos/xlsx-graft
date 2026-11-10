#!/bin/sh
# The license files in a directory that is released apart from the repository
# (sdk/go, which the Go proxy zips alone and a module zip cannot hold symbolic
# links; sdk/php, which is split into a repository of its own for Packagist)
# must be byte-for-byte equal to the files at the root of this repository, and
# be tracked by git (a release is made from the commit, not from the working
# tree).
#
#   scripts/check-license-copies.sh <directory>...
set -eu

if [ $# -eq 0 ]; then
    echo "usage: $0 <directory>..." >&2
    exit 2
fi

root=$(cd "$(dirname "$0")/.." && pwd)
failed=0
for dir in "$@"; do
    for name in LICENSE-APACHE LICENSE-BSD LICENSE-MIT NOTICE; do
        if ! cmp -s "$root/$name" "$root/$dir/$name"; then
            echo "license copies: $dir/$name is missing or not the repository's $name" >&2
            failed=1
        elif ! git -C "$root" ls-files --error-unmatch "$dir/$name" >/dev/null 2>&1; then
            echo "license copies: $dir/$name is not tracked by git" >&2
            failed=1
        fi
    done
done
[ "$failed" -eq 0 ] || exit 1
echo "license copies: $* equal the repository's"
