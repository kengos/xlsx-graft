#!/bin/sh
# A toolchain image holds the versions the Makefile pins, or the versions
# TOOLCHAIN_HOLDS says the pinned digest holds.
#
#   scripts/check-toolchain-image.sh [--holds '<versions>'] <image> [<platform>]
#
# Runs the image once and compares what each tool reports. Without --holds the
# versions are compared with RUST_VERSION, LLVM_COV_VERSION,
# CARGO_ABOUT_VERSION and CARGO_DENY_VERSION in the Makefile. With --holds they
# are compared with the given text, in the form this script prints after
# `holds:` (rust X, cargo-llvm-cov X, cargo-about X, cargo-deny X).
# engine:verify passes TOOLCHAIN_HOLDS as this text when it checks the image
# at TOOLCHAIN_DIGEST.
# engine:verify does not compare with the pins, because a bump changes the
# pins one pull request before TOOLCHAIN_DIGEST, and the gates would fail in
# between.
# <platform> (linux/amd64, linux/arm64) picks one architecture of a multi-arch
# image; without it docker runs the host's architecture.
#
# With or without --holds, the image must also have the git command (cargo-deny runs it to
# fetch the advisory database); its version is not compared.
#
# .github/workflows/toolchain-image.yml runs it over each architecture of
# the image it has just pushed, so an image missing a tool fails that run before
# anyone copies its digest into TOOLCHAIN_DIGEST.
# scripts/check-toolchain-pins.sh checks that the Makefile and the Dockerfile
# agree; this script checks what the built image holds.
#
# Read-only POSIX sh + sed + docker; no toolchain required on the host.
set -eu

usage() {
    echo "usage: $0 [--holds '<versions>'] <image> [<platform>]" >&2
    exit 2
}

mode=pins
holds=
if [ "${1:-}" = --holds ]; then
    [ $# -ge 2 ] || usage
    if [ -z "$2" ]; then
        echo "toolchain-image: --holds got an empty value (is TOOLCHAIN_HOLDS in the Makefile empty?)" >&2
        exit 2
    fi
    mode=holds
    holds=$2
    shift 2
fi
if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    usage
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
    echo "CARGO_DENY_VERSION=$(v cargo deny --version | cut -d" " -f2)"
    echo "GIT_VERSION=$(v git --version | cut -d" " -f3)"')

field() { printf '%s\n' "$have" | sed -n "s/^$1=//p"; }

where="$image${platform:+ ($platform)}"
status=0
text="rust $(field RUST_VERSION), cargo-llvm-cov $(field LLVM_COV_VERSION), cargo-about $(field CARGO_ABOUT_VERSION), cargo-deny $(field CARGO_DENY_VERSION)"
if [ -z "$(field GIT_VERSION)" ]; then
    echo "toolchain-image: git is missing from $where" >&2
    status=1
fi
if [ "$mode" = holds ]; then
    if [ "$text" != "$holds" ]; then
        echo "toolchain-image: $where holds '$text', but TOOLCHAIN_HOLDS says '$holds'" >&2
        status=1
    fi
else
    for name in RUST_VERSION LLVM_COV_VERSION CARGO_ABOUT_VERSION CARGO_DENY_VERSION; do
        want=$(make_pin "$name")
        got=$(field "$name")
        if [ -z "$want" ]; then
            echo "toolchain-image: $name not found in the Makefile" >&2
            status=1
        elif [ "$got" != "$want" ]; then
            echo "toolchain-image: $name is '$got' in $where, want '$want'" >&2
            status=1
        fi
    done
fi
[ "$status" -eq 0 ] || exit 1

echo "toolchain-image: OK ($image${platform:+ $platform}) holds: $text"
