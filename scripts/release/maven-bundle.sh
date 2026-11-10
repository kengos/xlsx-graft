#!/bin/sh
# The bundle Maven Central's Portal takes: the files of one version of
# io.github.kengos:xlsx-graft from a Maven repository, each with its .md5 and
# .sha1 checksums and a detached GPG signature (.asc), in a zip whose paths are
# the Maven repository's (io/github/kengos/xlsx-graft/<version>/...).
#
#   scripts/release/maven-bundle.sh <the Maven repository> <the bundle .zip>
#
# gpg signs with the key of GNUPGHOME: the release's key, imported from a secret
# (GPG_KEY_ID names it, and GPG_PASSPHRASE_FILE is a file holding its
# passphrase), or in a dry run a key generated for the run, which is the only
# one and has no passphrase (deleted when the container exits).
# The signatures are checked before the bundle is made. Needs gpg and the JDK's
# `jar`, which makes the zip.
set -eu

if [ $# -ne 2 ]; then
    echo "usage: $0 <maven repository> <bundle.zip>" >&2
    exit 2
fi
repo=$1
bundle=$2

dirs=$(find "$repo/io/github/kengos/xlsx-graft" -mindepth 1 -maxdepth 1 -type d)
if [ "$(printf '%s\n' "$dirs" | grep -c .)" -ne 1 ]; then
    echo "maven-bundle: $repo/io/github/kengos/xlsx-graft holds the version directories '$dirs', not exactly one" >&2
    exit 1
fi
dir=$dirs

work=$(mktemp -d)
mkdir -p "$work/io/github/kengos/xlsx-graft"
cp -r "$dir" "$work/io/github/kengos/xlsx-graft/"
cd "$work"

# The repository's metadata, and the checksums and signatures a publish to the
# repository made, are not part of the bundle: they are made again below.
find . -type f \( -name 'maven-metadata*' -o -name '*.md5' -o -name '*.sha1' \
    -o -name '*.sha256' -o -name '*.sha512' -o -name '*.asc' \) -delete
sign_options=
[ -z "${GPG_KEY_ID:-}" ] || sign_options="--local-user $GPG_KEY_ID"
[ -z "${GPG_PASSPHRASE_FILE:-}" ] || sign_options="$sign_options --pinentry-mode loopback --passphrase-file $GPG_PASSPHRASE_FILE"
for file in $(find io -type f | sort); do
    md5sum "$file" | cut -d' ' -f1 | tr -d '\n' > "$file.md5"
    sha1sum "$file" | cut -d' ' -f1 | tr -d '\n' > "$file.sha1"
    # shellcheck disable=SC2086 # $sign_options is a list of words
    gpg --batch --yes --armor --detach-sign $sign_options --output "$file.asc" "$file"
    gpg --batch --verify "$file.asc" "$file" \
        || { echo "maven-bundle: the signature of $file does not verify" >&2; exit 1; }
done

mkdir -p "$(dirname "$bundle")"
rm -f "$bundle"
jar --create --file "$bundle" --no-manifest -C "$work" io
echo "maven-bundle: $bundle"
