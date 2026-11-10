#!/bin/sh
# Prints the slug (platforms.tsv) of the Docker host's platform, and nothing
# when it is none of the release's: the platform whose engine
# `make release:native` builds.
set -eu

case "$(docker info --format '{{.Architecture}}')" in
    x86_64) echo linux-x64-gnu ;;
    aarch64|arm64) echo linux-arm64-gnu ;;
esac
