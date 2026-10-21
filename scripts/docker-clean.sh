#!/bin/sh
# Remove the Docker resources xlsx-graft's gates leave behind.
#
#   scripts/docker-clean.sh containers|volumes|images|all <toolchain-image>
#
# It never stops at the first thing it cannot remove: every resource ends up on
# a `removed` or a `kept` line with the reason, and `all` closes with the counts.
#
#   containers  exited or dead containers carrying the `xlsx-graft.tree` label,
#               from ANY working tree — a deleted worktree's leftovers included.
#               Every other state is only listed: a `created` one may be a gate
#               starting in another worktree. A running one whose tree has no
#               live gate (scripts/gate-lock.sh --state) is flagged as orphaned,
#               with the command that removes it.
#   volumes     the cargo download cache, and the per-Rust-version cargo/rustup
#               volumes gates made before the toolchain image (named
#               xlsx-graft-cargo-<version> / xlsx-graft-rustup-<version>). One
#               still mounted by a container is kept.
#   images      every pin of the toolchain image's repository
#               (<toolchain-image> is <repository>@<digest>): the current one
#               and those a bump left behind, and the SDK gate images
#               (xlsx-graft-sdk-<lang>:<version>-<hash>, built from
#               sdk/<lang>/Dockerfile and never published, so the next gate
#               builds them again).
#               Tagged rust:* images are listed, not removed: other
#               repositories may use them, and the gates used one before the
#               toolchain image.
set -u

job=${1:-}
image=${2:-}
removed=0
kept=0

say_removed() { echo "removed  $1"; removed=$((removed + 1)); }
say_kept() { echo "kept     $1 — $2"; kept=$((kept + 1)); }

clean_containers() {
    echo "== containers =="
    for id in $(docker ps -aq --filter label=xlsx-graft.tree); do
        # Gone between the listing and here (a --rm gate finishing): nothing
        # to report.
        info=$(docker inspect -f '{{.State.Status}} {{index .Config.Labels "xlsx-graft.tree"}}' "$id" 2>/dev/null) || continue
        state=${info%% *}
        tree=${info#* }
        case $state in
        running)
            case $("$(dirname "$0")/gate-lock.sh" --state "$tree") in
            live*) say_kept "container $id ($tree)" "running under a live gate" ;;
            stale*) say_kept "container $id ($tree)" "running, but its gate died (stale lock); orphaned: docker rm -f $id" ;;
            *) say_kept "container $id ($tree)" "running, and no gate lock found for its tree (one under another XLSX_GRAFT_GATE_DIR would not show); if nothing runs there: docker rm -f $id" ;;
            esac
            ;;
        exited | dead)
            if err=$(docker rm "$id" 2>&1); then
                say_removed "container $id ($tree)"
            else
                say_kept "container $id ($tree)" "$err"
            fi
            ;;
        *) say_kept "container $id ($tree)" "state $state, not stopped" ;;
        esac
    done
}

clean_volumes() {
    echo "== volumes =="
    for v in $(docker volume ls -q | grep -E '^xlsx-graft-(cargo|rustup)-'); do
        if err=$(docker volume rm "$v" 2>&1); then
            say_removed "volume $v"
        else
            say_kept "volume $v" "$err"
        fi
    done
}

clean_images() {
    echo "== images =="
    repo=${image%@*}
    # A digest pin is pulled untagged, and Docker 29's `docker images` lists an
    # untagged image only under -a. Removed by its references in this
    # repository rather than by ID: an ID with several references refuses
    # `docker rmi <id>`, and a name someone else gave the same image stays.
    for id in $(docker images -aq "$repo" | sort -u); do
        all=$(docker image inspect -f '{{range .RepoDigests}}{{.}} {{end}}{{range .RepoTags}}{{.}} {{end}}' "$id" 2>/dev/null) || continue
        refs=
        for ref in $all; do
            case $ref in
            "$repo"@* | "$repo":*) refs="$refs${refs:+ }$ref" ;;
            esac
        done
        [ -n "$refs" ] || continue
        # shellcheck disable=SC2086 # one word per reference
        if err=$(docker rmi $refs 2>&1); then
            say_removed "image $refs"
        else
            say_kept "image $refs" "$err"
        fi
    done
    for ref in $(docker images --format '{{.Repository}}:{{.Tag}}' --filter 'reference=xlsx-graft-sdk-*' | grep -v ':<none>$'); do
        if err=$(docker rmi "$ref" 2>&1); then
            say_removed "image $ref"
        else
            say_kept "image $ref" "$err"
        fi
    done
    for other in $(docker images --format '{{.Repository}}:{{.Tag}}' rust | grep -v ':<none>$'); do
        say_kept "image $other" "not this repository's image; other repositories may use it (docker rmi $other)"
    done
}

case $job in
images | all)
    # Anything but a digest pin is a caller's mistake, and an empty one would
    # turn the image listing into every image on the host: stop before it.
    case $image in
    ?*@sha256:?*) ;;
    *)
        echo "usage: $0 $job <repository>@sha256:<digest>" >&2
        exit 2
        ;;
    esac
    ;;
esac

case $job in
containers | volumes | images | all)
    # An unreachable daemon would otherwise read as "nothing to remove".
    if ! docker info >/dev/null 2>&1; then
        echo "docker is not reachable; nothing was checked" >&2
        exit 1
    fi
    ;;
esac

case $job in
containers) clean_containers ;;
volumes) clean_volumes ;;
images) clean_images ;;
all)
    # Containers first: a stopped container still holds its volumes.
    clean_containers
    clean_volumes
    clean_images
    echo "== summary: $removed removed, $kept kept =="
    ;;
*)
    echo "usage: $0 containers|volumes|images|all <toolchain-image>" >&2
    exit 2
    ;;
esac
