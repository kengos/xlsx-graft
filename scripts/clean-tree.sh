#!/bin/sh
# Remove what the gates write inside this working tree.
#
#   scripts/clean-tree.sh target|all
#
#   target  engine/target
#   all     engine/target, engine/lcov.info, dist/ (engine:capi-lib's output)
#           and .make-logs/ — the gate logs, the half-written <log>.<pid> of
#           an interrupted run, and a gate lock left behind by a gate that no
#           longer runs
#
# Runs under the gate lock, so it never deletes a build a running gate is
# using. Both modes also refuse while this tree's container runs without a live
# gate (a gate killed after its lock was released): it would keep writing
# engine/target after the delete. A lock whose owner is gone is stale
# (scripts/gate-lock.sh --state); `all` clears it first, since the lock would
# otherwise refuse the clean that is meant to remove it.
set -eu

root=$(git rev-parse --show-toplevel)
cd "$root"

state=$(scripts/gate-lock.sh --state)
case $state in
live*) ;; # gate-lock.sh refuses below, naming the owner (`all` also refuses under a held lock)
*)
    running=$(docker ps -q --filter "label=xlsx-graft.tree=$root" --filter status=running 2>/dev/null || true)
    if [ -n "$running" ]; then
        echo "this tree's container still runs without a gate (it would keep writing engine/target): docker rm -f $running" >&2
        exit 1
    fi
    ;;
esac

case ${1:-} in
target)
    exec scripts/gate-lock.sh rm -rf engine/target
    ;;
all)
    # Under a held lock (`make quiet T=clean:all`) gate-lock.sh lets this
    # through, and the delete below would take the running caller's log with it.
    if [ "${XLSX_GRAFT_GATE_HELD:-}" = "1" ]; then
        echo "clean:all removes .make-logs/, including the log of the run that started it: run \`make clean:all\` directly, not under quiet or a gate" >&2
        exit 1
    fi
    case $state in
    stale*)
        # Re-checked when clearing: the lock may have changed hands meanwhile.
        state=$(scripts/gate-lock.sh --clear-stale)
        case $state in
        stale*) echo "removed  stale gate lock (pid ${state#stale } is gone)" ;;
        esac
        ;;
    esac
    # Everything in .make-logs except gates/, which holds this run's own lock
    # until gate-lock.sh releases it.
    exec scripts/gate-lock.sh sh -euc '
        rm -rf engine/target engine/lcov.info dist
        if [ -d .make-logs ]; then
            find .make-logs -mindepth 1 -maxdepth 1 ! -name gates -exec rm -rf {} +
        fi
        echo "removed  engine/target, engine/lcov.info, dist/, .make-logs/"'
    ;;
*)
    echo "usage: $0 target|all" >&2
    exit 2
    ;;
esac
