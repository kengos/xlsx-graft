#!/bin/sh
# The toolchain image's build-arg defaults equal the Makefile's pins.
#
#   scripts/check-toolchain-pins.sh            check, print OK or the mismatch
#   scripts/check-toolchain-pins.sh --github   also append the pins to
#                                              $GITHUB_OUTPUT (rust=, llvm_cov=)
#
# The versions are written twice: RUST_VERSION and LLVM_COV_VERSION in the
# Makefile (what the workflow builds) and the ARG defaults in
# docker/toolchain.Dockerfile (what a plain `docker build` of it makes). The
# publishing workflow builds with the Makefile's values, so a mismatch would
# not make a wrong image; it would make a file that says one version while the
# image holds another. Refusing to publish is how a half-done bump is noticed.
#
# Only the exact spellings below are read (`NAME := x` in the Makefile,
# `ARG NAME=x` in the Dockerfile); a pin that moved to another spelling reads
# as missing, which fails rather than passing on an empty string.
#
# engine:budget runs it too, so a half-done bump fails in the pull request
# rather than in the publishing workflow after the merge. Whether the pinned
# digest holds these versions is engine:verify's toolchain step, not this.
#
# Pure read-only POSIX sh + standard tools; no toolchain required.
set -eu

case ${1:-} in
'' | --github) ;;
*)
    echo "usage: $0 [--github]" >&2
    exit 2
    ;;
esac

root=$(dirname "$0")/..
makefile=$root/Makefile
dockerfile=$root/docker/toolchain.Dockerfile

make_pin() { sed -n "s/^$1[[:blank:]]*:=[[:blank:]]*\([^[:blank:]#]*\).*/\1/p" "$makefile" | head -1; }
docker_pin() { sed -n "s/^ARG[[:blank:]][[:blank:]]*$1=\([^[:blank:]#]*\).*/\1/p" "$dockerfile" | head -1; }

status=0
for name in RUST_VERSION LLVM_COV_VERSION; do
    m=$(make_pin "$name")
    d=$(docker_pin "$name")
    if [ -z "$m" ] || [ -z "$d" ]; then
        echo "toolchain-pins: $name not found (Makefile: '${m}', Dockerfile: '${d}')" >&2
        status=1
    elif [ "$m" != "$d" ]; then
        echo "toolchain-pins: $name is $m in the Makefile but $d in docker/toolchain.Dockerfile; bump both" >&2
        status=1
    fi
done
[ "$status" -eq 0 ] || exit 1

rust=$(make_pin RUST_VERSION)
llvm_cov=$(make_pin LLVM_COV_VERSION)
if [ "${1:-}" = --github ]; then
    printf 'rust=%s\nllvm_cov=%s\n' "$rust" "$llvm_cov" >>"$GITHUB_OUTPUT"
fi
echo "toolchain-pins: OK (rust $rust, cargo-llvm-cov $llvm_cov)"
