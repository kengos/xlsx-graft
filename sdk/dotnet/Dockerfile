# The .NET SDK's gate environment: `make sdk:*:dotnet` builds and runs this
# image, so nobody needs .NET installed. The build context is sdk/dotnet.
#
# The image holds the .NET SDK and the locked test tools in its package cache,
# and nothing else. The engine library is NOT copied in: the gate mounts the
# repository at /repo and points XLSX_GRAFT_LIBRARY at the library `make
# engine:capi-lib` built there, so changing the engine never rebuilds this
# image. That library is built on bookworm (glibc 2.36). Microsoft ships no
# Debian image of .NET 10, so this is noble (glibc 2.39): glibc loads a library
# built against the same or an older glibc, and alpine (musl) would load
# nothing.
#
# DOTNET_VERSION has no default on purpose. mk/sdk/dotnet.mk maps DOTNET_VER
# (10.0) to the SDK release, and it is the one place that number is written.
ARG DOTNET_VERSION

# `sdk:lock:dotnet` runs this stage alone: it has to write the lockfiles, so it
# cannot restore from them.
FROM mcr.microsoft.com/dotnet/sdk:${DOTNET_VERSION}-noble AS sdk

# Nothing a gate runs writes into the mounted checkout, which it would leave
# owned by root: bin/ and obj/ go to /tmp (ArtifactsPath), and the package
# cache lives in the image, outside /repo (the mount would hide it).
ENV DOTNET_CLI_TELEMETRY_OPTOUT=1 \
    DOTNET_NOLOGO=1 \
    DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1 \
    NUGET_PACKAGES=/opt/nuget \
    ArtifactsPath=/tmp/artifacts

FROM sdk

# Only the files that decide the packages. Editing a .cs file then never
# re-runs the restore. Locked mode (Directory.Build.props): a csproj edit
# without `make sdk:lock:dotnet` fails here rather than gating on the stale
# lock. The gate's own restores then find every package in /opt/nuget and
# fetch nothing.
ENV XLSX_GRAFT_LOCKED_RESTORE=true
WORKDIR /locked
COPY Directory.Build.props ./
COPY XlsxGraft/XlsxGraft.csproj XlsxGraft/packages.lock.json XlsxGraft/
COPY XlsxGraft.Tests/XlsxGraft.Tests.csproj XlsxGraft.Tests/packages.lock.json XlsxGraft.Tests/
RUN dotnet restore XlsxGraft.Tests/XlsxGraft.Tests.csproj

WORKDIR /repo/sdk/dotnet
