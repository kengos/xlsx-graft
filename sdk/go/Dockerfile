# The Go SDK's gate environment: `make sdk:*:go` builds and runs this image,
# so nobody needs Go installed. The build context is sdk/go.
#
# The image holds Go and golangci-lint, and nothing else; the SDK has no
# dependencies. The engine is NOT copied in: the gate mounts the repository at
# /repo and points XLSX_GRAFT_CLI at the CLI `make engine:cli-bin` built there,
# so changing the engine never rebuilds this image. That CLI is built on
# bookworm, so this is bookworm too: a binary linked against glibc runs on the
# same or a newer glibc, and on a musl base not at all. The bookworm image
# carries gcc, which `go test -race` needs.
#
# GO_PATCH has no default on purpose. mk/sdk/go.mk maps GO_VER (1.26, …) to
# the patch release, and it is the one place that number is written.
ARG GO_PATCH

# `sdk:lock:go` runs this stage alone: it has to make tools/go.sum, so it
# cannot build from it.
FROM golang:${GO_PATCH}-bookworm AS tools

# GOTOOLCHAIN=local: a go.mod asking for a newer Go fails here rather than
# having another toolchain downloaded. The caches sit outside /repo, which
# the gate mounts over.
ENV GOTOOLCHAIN=local \
    GOMODCACHE=/opt/go/mod \
    GOCACHE=/opt/go/cache

FROM tools

# Only the files that pin the tools. Editing the SDK then never re-runs the
# build. golangci-lint is built from the module tools/go.sum locks, on this
# line's Go, so it reads the code the way this Go does. The build cache it
# leaves is dropped; the module cache is not needed after it either.
WORKDIR /tools
COPY tools/go.mod tools/go.sum ./
RUN go build -mod=readonly -o /usr/local/bin/golangci-lint github.com/golangci/golangci-lint/v2/cmd/golangci-lint \
  && go clean -cache -modcache

# The gate reaches no network: the SDK has no dependencies, and the package
# step builds a user's module from a local `replace` alone (mk/sdk/go.mk).
ENV GOFLAGS=-mod=readonly \
    GOPROXY=off

WORKDIR /repo/sdk/go
