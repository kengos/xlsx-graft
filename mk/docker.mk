# docker/ — the Docker resources the gates use: containers, cache volumes, the
# toolchain image and the SDK gate images
#
# Every target defined here is named `docker:<job>`. They remove things and
# prove nothing, so they do not go through `quiet`: each resource is reported
# on a `removed` or `kept` line (scripts/docker-clean.sh says what is kept and
# why). The files inside a working tree are `clean:*`, kept apart on purpose.

.PHONY: docker\:clean\:all docker\:clean\:containers docker\:clean\:images \
        docker\:clean\:volumes

docker\:clean\:containers: ## Remove stopped gate containers from any worktree (running ones are only listed)
	@scripts/docker-clean.sh containers $(RUST_IMAGE)

docker\:clean\:volumes: ## Remove the cargo download cache volume (and the per-Rust-version cargo/rustup volumes of the old layout)
	@scripts/docker-clean.sh volumes $(RUST_IMAGE)

docker\:clean\:images: ## Remove every pin of the toolchain image and the SDK gate images (rust:* images are only listed)
	@scripts/docker-clean.sh images $(RUST_IMAGE)

docker\:clean\:all: ## All three above, reporting what was removed and what was kept
	@scripts/docker-clean.sh all $(RUST_IMAGE)
