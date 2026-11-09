# The toolchain image the gates run in: rust with the components and the cargo
# subcommands the gates need, the license tools (cargo-about, cargo-deny) and
# git, installed once here instead of into a cache volume on each machine.
# .github/workflows/toolchain-image.yml publishes it as
# ghcr.io/<owner>/xlsx-graft-toolchain.
#
# Every ARG default below MUST equal the Makefile variable of the same name,
# which is what the workflow builds with (the gates run the digest the
# Makefile pins, TOOLCHAIN_DIGEST). The workflow passes the Makefile's values
# as build args, and it and engine:budget both fail when these defaults differ
# (scripts/check-toolchain-pins.sh), so a plain `docker build` of this file
# and the published image cannot quietly be two different toolchains.
#
# No crate of the workspace is built here: dependencies change with every
# lockfile edit, and an image that went stale with them would be rebuilt for
# nothing the toolchain cares about.
ARG RUST_VERSION=1.99.0
FROM rust:${RUST_VERSION}-slim-bookworm

ARG LLVM_COV_VERSION=0.9.1

# One layer, so what is deleted at the end never reaches the image: the
# registry and git checkouts cargo install fetched, and rustup's download and
# temp directories. The installed binaries and cargo's install records
# (.crates.toml, .crates2.json) stay.
RUN rustup component add rustfmt clippy llvm-tools-preview \
    && cargo install cargo-llvm-cov --locked --version "${LLVM_COV_VERSION}" \
    && rm -rf "${CARGO_HOME}/registry" "${CARGO_HOME}/git" \
        "${RUSTUP_HOME}/downloads" "${RUSTUP_HOME}/tmp"

ARG CARGO_ABOUT_VERSION=0.9.3
ARG CARGO_ABOUT_SHA256_x86_64=3ad1e94ea1e63d69d1961f4fb15ccbbe9575d906111f96d2cb4dce6df48f71b4
ARG CARGO_ABOUT_SHA256_aarch64=eb4d9ff6ebd2518152a8308729431a04b3836e71d2bd5ead7aea69f91a71162e
ARG CARGO_DENY_VERSION=0.20.2
ARG CARGO_DENY_SHA256_x86_64=9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f
ARG CARGO_DENY_SHA256_aarch64=995c82be0defc7a025cae49a2aa2644ce8245c9a3318fc4103907c6a285e8c7d

# cargo-about and cargo-deny as the upstream RELEASE tarballs, each checked
# against the checksum pinned for this architecture before anything is
# unpacked: a version tag alone is not an integrity check. Release binaries
# rather than `cargo install`, because the arm64 image is built under QEMU,
# where compiling these two crates from source is far slower than downloading
# them. Only the one executable of each tarball is kept. curl is installed and
# purged within this one RUN, so the image does not contain it.
RUN set -eu; \
    case "$(uname -m)" in \
        x86_64) arch=x86_64 \
            about_sha=${CARGO_ABOUT_SHA256_x86_64} \
            deny_sha=${CARGO_DENY_SHA256_x86_64} ;; \
        aarch64) arch=aarch64 \
            about_sha=${CARGO_ABOUT_SHA256_aarch64} \
            deny_sha=${CARGO_DENY_SHA256_aarch64} ;; \
        *) echo "no pinned tool checksums for $(uname -m)" >&2; exit 1 ;; \
    esac; \
    apt-get update -qq; \
    apt-get install -y -qq --no-install-recommends curl ca-certificates >/dev/null; \
    fetch() { \
        curl -fsSL -o /tmp/tool.tar.gz "$1"; \
        echo "$2  /tmp/tool.tar.gz" | sha256sum -c -; \
        tar --no-same-owner -xzf /tmp/tool.tar.gz -C /usr/local/bin \
            --strip-components=1 "$3"; \
        rm /tmp/tool.tar.gz; \
    }; \
    about=cargo-about-${CARGO_ABOUT_VERSION}-${arch}-unknown-linux-musl; \
    fetch "https://github.com/EmbarkStudios/cargo-about/releases/download/${CARGO_ABOUT_VERSION}/${about}.tar.gz" \
        "$about_sha" "${about}/cargo-about"; \
    deny=cargo-deny-${CARGO_DENY_VERSION}-${arch}-unknown-linux-musl; \
    fetch "https://github.com/EmbarkStudios/cargo-deny/releases/download/${CARGO_DENY_VERSION}/${deny}.tar.gz" \
        "$deny_sha" "${deny}/cargo-deny"; \
    apt-get purge -y -qq --auto-remove curl >/dev/null; \
    rm -rf /var/lib/apt/lists/*

# git, for `cargo deny check advisories`: the cargo-deny release binary
# downloads the RustSec advisory database by running the git command, and
# fails without it (checked with cargo-deny 0.20.2, also with
# `git-fetch-with-cli = false`). The version is whatever Debian bookworm ships
# and is not pinned: git only downloads the database, so the result of a gate
# depends on the database's contents and not on the git version, and a pin
# would need a bump for every Debian security update. ca-certificates is
# installed by name because `--no-install-recommends` leaves out what git only
# recommends, and so that this layer does not depend on the layer above (which
# installs and removes curl). This is the last layer, so a change here keeps
# the layers above in the build cache.
RUN apt-get update -qq \
    && apt-get install -y -qq --no-install-recommends git ca-certificates >/dev/null \
    && rm -rf /var/lib/apt/lists/*
