# The toolchain image the gates run in: rust with the components and the cargo
# subcommand the gates need, installed once here instead of into a cache
# volume on each machine. .github/workflows/toolchain-image.yml publishes it as
# ghcr.io/<owner>/xlsx-graft-toolchain.
#
# The defaults below MUST equal RUST_VERSION and LLVM_COV_VERSION in the
# Makefile, which is what the gates run today. The workflow passes the
# Makefile's values as build args and refuses to publish when these defaults
# differ (scripts/check-toolchain-pins.sh), so a plain `docker build` of this
# file and the published image cannot quietly be two different toolchains.
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
