#!/bin/sh
# A toolchain image holds the versions the Makefile pins.
#
#   scripts/check-toolchain-image.sh <image> [<platform>]
#
# Runs the image once and compares what each tool reports with RUST_VERSION,
# LLVM_COV_VERSION, CARGO_ABOUT_VERSION and CARGO_DENY_VERSION in the
# Makefile. <platform> (linux/amd64, linux/arm64) picks one architecture of
# a multi-arch image; without it docker runs the host's architecture.
#
# .github/workflows/toolchain-image.yml runs it over each architecture of the
# image it has just pushed, so an image missing a tool fails that run before
# anyone copies its digest into TOOLCHAIN_DIGEST.
# scripts/check-toolchain-pins.sh checks that the Makefile and the Dockerfile
# agree; this script checks that the built image holds the Makefile's
# versions.
#
# Read-only POSIX sh + sed + docker; no toolchain required on the host.
set -eu

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    echo "usage: $0 <image> [<platform>]" >&2
    exit 2
fi
image=$1
platform=${2:-}

makefile=$(dirname "$0")/../Makefile
make_pin() { sed -n "s/^$1[[:blank:]]*:=[[:blank:]]*\([^[:blank:]#]*\).*/\1/p" "$makefile" | head -1; }

# A tool that is missing or fails prints an empty version, which then fails
# the comparison below by name rather than aborting with no word of which.
have=$(docker run --rm ${platform:+--platform "$platform"} "$image" sh -c '
    v() { "$@" 2>/dev/null | head -1; }
    echo "RUST_VERSION=$(v rustc --version | cut -d" " -f2)"
    echo "LLVM_COV_VERSION=$(v cargo llvm-cov --version | cut -d" " -f2)"
    echo "CARGO_ABOUT_VERSION=$(v cargo about --version | cut -d" " -f2)"
    echo "CARGO_DENY_VERSION=$(v cargo deny --version | cut -d" " -f2)"')

field() { printf '%s\n' "$have" | sed -n "s/^$1=//p"; }

status=0
for name in RUST_VERSION LLVM_COV_VERSION CARGO_ABOUT_VERSION CARGO_DENY_VERSION; do
    want=$(make_pin "$name")
    got=$(field "$name")
    if [ -z "$want" ]; then
        echo "toolchain-image: $name not found in the Makefile" >&2
        status=1
    elif [ "$got" != "$want" ]; then
        echo "toolchain-image: $name is '$got' in $image${platform:+ ($platform)}, want '$want'" >&2
        status=1
    fi
done
[ "$status" -eq 0 ] || exit 1

echo "toolchain-image: OK ($image${platform:+ $platform}: rust $(field RUST_VERSION), cargo-llvm-cov $(field LLVM_COV_VERSION), cargo-about $(field CARGO_ABOUT_VERSION), cargo-deny $(field CARGO_DENY_VERSION))"
