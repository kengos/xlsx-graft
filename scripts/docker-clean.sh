#!/bin/sh
# Remove the Docker resources xlsx-graft's gates leave behind.
#
#   scripts/docker-clean.sh containers|volumes|images|all <rust-image>
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
#   volumes     the cargo/rustup cache volumes of EVERY Rust version, so a
#               toolchain bump leaves nothing behind. One still mounted by a
#               container is kept.
#   images      the pinned toolchain image (<rust-image>). Other tagged rust:*
#               images are listed, not removed: other repositories may use them
#               — after a RUST_VERSION bump that includes the old pin.
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
    if [ -n "$(docker images -q "$image")" ]; then
        if err=$(docker rmi "$image" 2>&1); then
            say_removed "image $image"
        else
            say_kept "image $image" "$err"
        fi
    fi
    for other in $(docker images --format '{{.Repository}}:{{.Tag}}' rust | grep -v ':<none>$' | grep -vxF "$image"); do
        say_kept "image $other" "not this repository's pin; other repositories may use it (docker rmi $other)"
    done
}

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
    echo "usage: $0 containers|volumes|images|all <rust-image>" >&2
    exit 2
    ;;
esac
