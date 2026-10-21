#!/usr/bin/env bash
# Point at the lines that failed the 100% coverage gate.
#
# Reads the lcov.info `make engine:coverage` already wrote — it does NOT re-run the
# gate, so this is instant. `cargo llvm-cov` is invoked with --lcov, which
# writes the machine-readable report to a file and prints no per-file table, so
# a failing run otherwise says only "below the threshold" with no file named.
#
# Coverage counts each crate TWICE (its own unit-test binary + the copy linked
# into dependents' test binaries), so lcov holds several SF blocks per file.
# That split is reported separately: a line covered in one copy and not the
# other is the trap CLAUDE.md warns about, and it reads very differently from a
# line no test reaches at all.
set -euo pipefail

LCOV="${1:-engine/lcov.info}"

# `make engine:coverage` deletes the report before it runs, so "absent" here is
# informative rather than an inconvenience: the gate got far enough to try, and
# produced nothing. That is what a test that DOES NOT COMPILE (or FAILS) looks like, and
# it is the case this message exists for — the alternative was reading a stale
# report and naming lines that a compile error means nothing ever reached.
if [ ! -f "$LCOV" ]; then
	cat >&2 <<-MSG
		coverage-why: no $LCOV — the run wrote no report.

		  If 'make engine:coverage' (or engine:verify) just failed, it failed BEFORE measuring anything.
		  The usual causes are a test that does not compile or a test that fails,
		  and the real message is in the gate log:

		    grep -nE 'error\[|panicked at|test result: FAILED' .make-logs/last-error.log

		  If you have not run the gate yet, run 'make engine:coverage' — it writes the
		  report this script reads.
	MSG
	exit 2
fi

awk '
/^SF:/ {
	file = substr($0, 4)
	sub(/^\/repo\//, "", file)
	next
}
/^DA:/ {
	split(substr($0, 4), a, ",")
	line = a[1]; count = a[2]
	key = file SUBSEP line
	seen[key] = 1
	total[key] += count
	if (count == 0) zero[key] += 1; else nonzero[key] += 1
	files[file] = 1
	next
}
END {
	ndead = 0; nsplit = 0
	for (key in seen) {
		split(key, p, SUBSEP)
		f = p[1]; l = p[2]
		if (total[key] == 0) {
			dead[f] = dead[f] " " l; ndead++
		} else if (zero[key] > 0 && nonzero[key] > 0) {
			mixed[f] = mixed[f] " " l; nsplit++
		}
	}
	if (ndead == 0 && nsplit == 0) {
		print "coverage-why: every line is covered in every instantiation."
		exit 0
	}
	if (ndead > 0) {
		printf "UNCOVERED — no test reaches these (%d lines):\n", ndead
		for (f in dead) printf "  %s:%s\n", f, substr(dead[f], 2)
	}
	if (nsplit > 0) {
		if (ndead > 0) print ""
		printf "COVERED IN ONE COPY ONLY — the double-count trap (%d lines):\n", nsplit
		printf "  (covered via a dependent'"'"'s suite but not the crate'"'"'s own tests,\n"
		printf "   or vice versa; cover it in the crate'"'"'s OWN unit tests)\n"
		for (f in mixed) printf "  %s:%s\n", f, substr(mixed[f], 2)
	}
}
' "$LCOV"
