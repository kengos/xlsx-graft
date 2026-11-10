#!/bin/bash
# Builds the engine of one macOS or Windows platform on its own runner: the C
# ABI library, the CLI and the Node.js addon, into dist/release/native/<slug>/.
#
#   scripts/release/build-native.sh <slug>
#
# Run by .github/workflows/release-artifacts.yml. The Linux platforms are built
# in the pinned toolchain image by `make release:native`; Apple's toolchain and
# MSVC do not run in a Linux container, and the hosted macOS and Windows
# runners have no Docker for Linux containers, so these two build with rustup
# at the Rust version the Makefile pins (RUST_VERSION), natively, never as a
# cross build. Not run by any gate: the platform it builds on is not the one a
# contributor's machine is.
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "usage: $0 <slug>" >&2
    exit 2
fi
slug=$1
root=$(cd "$(dirname "$0")/../.." && pwd)
tool="$root/scripts/release/platform.sh"

capi=$("$tool" "$slug" capi)
cli=$("$tool" "$slug" cli)
case "$slug" in
    darwin-*)
        addon_built=libxlsx_graft_napi.dylib
        # The oldest macOS the wheel tag macosx_11_0 promises.
        export MACOSX_DEPLOYMENT_TARGET=11.0
        ;;
    win32-*)
        addon_built=xlsx_graft_napi.dll
        # The C runtime linked in, so a machine without the Visual C++
        # redistributable loads the library. RUSTFLAGS replaces the rustflags
        # of a cargo configuration file; the repository has none (no
        # engine/.cargo/config.toml), so nothing is lost.
        export RUSTFLAGS="-C target-feature=+crt-static"
        ;;
    *)
        echo "build-native: $slug is built by make release:native, in the toolchain image" >&2
        exit 2
        ;;
esac

version=$(sed -n 's/^RUST_VERSION *:= *//p' "$root/Makefile")
if [ -z "$version" ]; then
    echo "build-native: no RUST_VERSION in the Makefile" >&2
    exit 1
fi
rustup toolchain install "$version" --profile minimal
rustup default "$version"

# The same cargo commands as engine:capi-lib, engine:napi and engine:cli-bin
# (mk/engine.mk): release profile, --locked, and the addon with its `shim`
# feature. Keep them equal, so no platform ships a different engine.
cd "$root/engine"
cargo build --release --locked -p xlsx-graft-capi
cargo build --release --locked -p xlsx-graft-cli --bin xlsx-graft
cargo build --release --locked -p xlsx-graft-napi --features shim

out="$root/dist/release/native/$slug"
rm -rf "$out"
mkdir -p "$out"
cp "target/release/$capi" "$out/"
cp "target/release/$cli" "$out/"
cp "target/release/$addon_built" "$out/xlsx_graft.node"

# The same check as make engine:cli-bin: the binary starts and prints the
# workspace's version.
want=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"$/\1/p' Cargo.toml)
json=$("$out/$cli" --version --format json)
case "$json" in
    *"\"version\":\"$want\""*) echo "$slug: cli prints $json" ;;
    *) echo "build-native: the CLI printed $json, but engine/Cargo.toml says $want" >&2; exit 1 ;;
esac
ls -l "$out"
