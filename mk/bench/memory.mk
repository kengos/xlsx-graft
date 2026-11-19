# bench:memory — the gate on the CLI's peak memory
#
# Unlike the rest of bench (mk/bench.mk), a gate and a member of `verify` and
# of `ci:matrix`: it prints one PASS/FAIL line, exits non-zero on FAIL and keeps
# its log in .make-logs/. It checks peak memory, never time, which changes with
# the machine and the day; a run that fails or reads the wrong number of rows
# fails it too.
#
# It runs the CLI's `read` and `verify` on fixtures/generated/shared-100000.xlsx
# and shared-1000000.xlsx (written by fixtures:generate) through
# bench/ruby/compare.rb: three runs each, the largest peak RSS of the runs by
# GNU `/usr/bin/time -v`, in the same container as bench:cli (30 minutes, 8 GiB).
# bench/ruby/memory.rb then checks the result against two promises:
#
# 1. The peak on 1,000,000 rows is at most BENCH_MEMORY_PEAK_MIB.
# 2. The peak on 1,000,000 rows is at most the peak on 100,000 rows plus
#    BENCH_MEMORY_GROWTH_MIB, the default sharedStringsSpillBytes: the shared
#    strings table may grow up to that before it moves to a temporary file.
#
# The limits are described in docs/benchmarks.md. Changing one needs the table
# there measured again, and a reason in the pull request. They are set with `:=`
# so that a value given on the command line appears in the PASS/FAIL line.

BENCH_MEMORY_PEAK_MIB := 64
BENCH_MEMORY_GROWTH_MIB := 32

.PHONY: bench\:memory _bench-memory

bench\:memory: ## Gate: the CLI's read and verify peak RSS on 1,000,000 shared-string rows stays under BENCH_MEMORY_PEAK_MIB and its growth from 100,000 rows under BENCH_MEMORY_GROWTH_MIB (a member of verify)
	@$(call gate,_bench-memory,bench:memory)

_bench-memory: fixtures\:generate _engine-cli-bin _bench-image
	@echo "== bench:memory: measure =="
	$(BENCH_IN_DOCKER) bundle exec ruby bench/ruby/compare.rb --only cli --books shared-100000,shared-1000000 --out dist/bench/memory
	@echo "== bench:memory: check =="
	$(BENCH_IN_DOCKER) bundle exec ruby bench/ruby/memory.rb dist/bench/memory.json --peak-mib $(BENCH_MEMORY_PEAK_MIB) --growth-mib $(BENCH_MEMORY_GROWTH_MIB)
