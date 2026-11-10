#!/bin/sh
# Restores the .NET package of a release into a project of its own and runs it
# with no XLSX_GRAFT_LIBRARY, so the only engine it can reach is the library the
# package carries under runtimes/<rid>/native/. Runs inside the .NET gate
# image.
#
#   scripts/release/smoke-dotnet.sh
set -eu

unset XLSX_GRAFT_LIBRARY
mkdir /tmp/pkg
cp /repo/dist/release/packages/dotnet/*.nupkg /tmp/pkg/
cp -r /repo/sdk/dotnet/script/CheckInstalledPackage /tmp/user
cp /repo/sdk/dotnet/.editorconfig /tmp/user/
cd /tmp/user
NUGET_PACKAGES=/tmp/user-packages dotnet run -c Release -- /tmp/user-packages /repo/engine/Cargo.toml release
