#!/bin/sh
# One gate at a time, per working tree.
#
# Two gates running in ONE tree corrupt each other: they share one cargo target
# (engine/target), and the tell is a cargo error blaming a test rather than your
# code ("the binary the runner was about to execute was deleted underneath it").
# That is easy to hit from an agent harness, where a "quick" second command
# lands while a long gate is still going in the background.
#
# The lock is keyed by WORKING TREE, not by machine: separate worktrees each
# have their own engine/target, so they may run gates in parallel.
#
#   scripts/gate-lock.sh <command> [args...]
#
# Re-entrant: a nested gate (`make quiet T=engine:verify` invoking make again)
# inherits the hold through XLSX_GRAFT_GATE_HELD and does not deadlock.
#
# XLSX_GRAFT_GATE_DIR puts the marker somewhere shared, so `ls` shows every
# running gate across every worktree at once. It defaults to a path inside this
# tree, which needs no configuration and still locks correctly.
set -eu

if [ "${XLSX_GRAFT_GATE_HELD:-}" = "1" ]; then
  exec "$@"
fi

tree_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
# A stable, filesystem-safe id for this working tree.
tree_id=$(printf '%s' "$tree_root" | tr '/ ' '__' | sed 's/^_*//')
gate_dir="${XLSX_GRAFT_GATE_DIR:-$tree_root/.make-logs/gates}"
lock="$gate_dir/$tree_id.running"

mkdir -p "$gate_dir"

# mkdir is atomic on POSIX filesystems: exactly one racer creates it.
if ! mkdir "$lock" 2>/dev/null; then
  echo "another gate is already running in this working tree:" >&2
  if [ -f "$lock/owner" ]; then
    sed 's/^/  /' "$lock/owner" >&2
  else
    echo "  (no owner recorded — the holder may have died mid-write)" >&2
  fi
  echo >&2
  echo "  Wait for it, or if you are certain nothing is running:" >&2
  echo "    rm -rf '$lock'" >&2
  exit 1
fi

# Release on every exit path, including SIGINT/SIGTERM — a Ctrl-C'd gate that
# left its lock behind would block the tree until someone read this file. Set
# right after the mkdir, so no window exists where the lock is held unguarded.
release() {
  rm -rf "$lock"
}
trap 'release' EXIT
trap 'release; exit 130' INT
trap 'release; exit 143' TERM

{
  echo "command : $*"
  echo "pid     : $$"
  echo "started : $(date '+%Y-%m-%d %H:%M:%S')"
  echo "tree    : $tree_root"
} > "$lock/owner"

XLSX_GRAFT_GATE_HELD=1
export XLSX_GRAFT_GATE_HELD

"$@"
