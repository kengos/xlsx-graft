# Benchmarks

What reading and verifying a large workbook costs in memory, measured
against the Ruby xlsx gems, and the limits `make bench:memory` holds the
engine to.

## The claim

Reading a sheet or verifying a workbook holds one row at a time, so the
peak memory does not grow with the row count. One thing does grow: the
shared strings table, which Excel writes for every workbook with text.
The reader keeps its text and its offsets in memory up to 32 MiB
(`sharedStringsSpillBytes`); past that it moves both to a temporary file
and keeps a write buffer of half that size for each in memory. A workbook
whose table passes 32 MiB therefore costs up to about 32 MiB more than a
small one, and no more as it grows.

## The workbooks

`make fixtures:generate` writes four workbooks into `fixtures/generated/`
(gitignored) with rust_xlsxwriter (`engine/fixtures-gen`). Each has one
sheet with a header row and that many data rows.

| Workbook | Rows | Values | Strings |
| --- | --- | --- | --- |
| `rows-100000.xlsx` | 100,000 | 5 columns: two numbers, a string, a boolean, a date | in the cells (`t="inlineStr"`), no shared strings table |
| `rows-1000000.xlsx` | 1,000,000 | the same | the same |
| `shared-100000.xlsx` | 100,000 | 3 columns: a number and two strings | every string different, in the shared strings table: about 10 MB, under 32 MiB |
| `shared-1000000.xlsx` | 1,000,000 | the same | the same: about 100 MB, past 32 MiB |

The shared books are the unfavourable case for the table: Excel stores a
string that repeats in many cells once, and these strings never repeat. `engine/fixtures-gen`'s
`write_rows_book` and `write_shared_book` say exactly what each holds.

## How it is measured

- **Peak memory** is the maximum resident set size GNU `/usr/bin/time -v`
  reports for the process: the same yardstick for every reader, taken from
  outside it. Each run is a new process. What the reader moved to a
  temporary file, and the page cache of reading the workbook, are not in
  it.
- **Time** is the elapsed (wall clock) time `/usr/bin/time` reports.
- Each pair of a workbook and a reader runs three times. The table shows
  the largest peak memory of the three and the median time.
- In brackets: what the reading adds to loading the reader's library
  alone (`require`, or the CLI's `--version`), measured the same way.
- Every reader takes every value of every row of the first sheet. The
  CLI's `read --format json` writes each row as a JSON line to a file; the
  Ruby readers count the rows and values and print the counts. A run
  counts only when the counts are the workbook's.
- Every run is in a container of 8 GiB without swap, and is stopped after
  30 minutes. A reader that needs more memory is killed and shown as
  `killed past 8 GiB`.

The Ruby SDK rows run in a Ruby process like the gems, so they carry
Ruby's own start and the SDK's loading (22 MiB). The CLI rows are the engine alone,
the same entry point the gate measures.

## Results

Measured on 2026-11-18 12:57 UTC with the measuring code of [#181](https://github.com/kengos/xlsx-graft/pull/181), the
pull request that added the shared workbooks and `bench/ruby` (at a
commit before it was rebased, whose measuring code differs from what was
merged only in the report's wording):

- Machine: Apple M3 Pro, 12 cores, 36 GB
- Docker Desktop 29.8.0, given 6 CPUs and 17.5 GiB; the container is
  aarch64 Linux, `nproc` 6, memory limit 8 GiB
- Ruby 4.0.7 (bookworm)
- rubyXL 3.4.38, roo 4.0.0, creek 2.6.3, xsv 1.4.1,
  simple_xlsx_reader 5.1.0; fiddle 1.1.8, nokogiri 1.19.7, rubyzip 3.7.0
  (`bench/ruby/Gemfile.lock`)

| Reader | rows-100000 | rows-1000000 | shared-100000 | shared-1000000 |
| --- | --- | --- | --- | --- |
| rubyXL | 1250 MiB (+1219) · 14.1 s | killed past 8 GiB | 983 MiB (+952) · 11.6 s | killed past 8 GiB |
| Roo | 105 MiB (+68) · 4.9 s | 109 MiB (+72) · 49.2 s | 300 MiB (+263) · 3.8 s | 2024 MiB (+1987) · 57.9 s |
| Creek | 457 MiB (+428) · 3.3 s | 4179 MiB (+4150) · 33.3 s | 376 MiB (+347) · 4.7 s | 4169 MiB (+4140) · 61.1 s |
| Xsv | 36 MiB (+12) · 3.6 s | 38 MiB (+14) · 35.4 s | 114 MiB (+90) · 3.4 s | 380 MiB (+356) · 45.6 s |
| simple_xlsx_reader | 31 MiB (+4) · 2.0 s | 31 MiB (+4) · 19.2 s | 117 MiB (+90) · 1.8 s | 525 MiB (+498) · 20.3 s |
| xlsx-graft Ruby SDK, read | 26 MiB (+4) · 1.0 s | 26 MiB (+4) · 9.8 s | 36 MiB (+14) · 0.9 s | 55 MiB (+33) · 9.7 s |
| xlsx-graft Ruby SDK, verify | 23 MiB (+1) · 0.5 s | 23 MiB (+1) · 4.0 s | 23 MiB (+1) · 0.5 s | 23 MiB (+1) · 4.0 s |
| xlsx-graft CLI, read | 3 MiB (+0) · 0.4 s | 3 MiB (+0) · 4.1 s | 13 MiB (+10) · 0.3 s | 35 MiB (+32) · 4.4 s |
| xlsx-graft CLI, verify | 4 MiB (+1) · 0.4 s | 4 MiB (+1) · 4.2 s | 4 MiB (+1) · 0.4 s | 4 MiB (+1) · 4.0 s |

| Library | Loading alone |
| --- | --- |
| rubyXL | 31 MiB · 0.1 s |
| Roo | 37 MiB · 0.1 s |
| Creek | 29 MiB · 0.1 s |
| Xsv | 24 MiB · 0.1 s |
| simple_xlsx_reader | 27 MiB · 0.1 s |
| xlsx-graft Ruby SDK | 22 MiB · 0.1 s |
| xlsx-graft CLI | 3 MiB · 0.0 s |

Verify has no column for the gems: none of them verifies a workbook.
The gems' figures move by up to a few hundred MiB between runs on the
same machine (their garbage collector decides when memory goes back); the
xlsx-graft figures move by a few MiB.

On GitHub's `ubuntu-latest` runner (x86_64, the image of 2026-11-11), the
CLI's `read` took 4, 4, 14 and 36 MiB on the four workbooks, and `verify`
4 MiB on each. CI's runners vary in speed, so its times are not shown.

## The gate: `make bench:memory`

`make bench:memory` is a member of `make verify` and a job of CI. It
runs the CLI's `read` and `verify` on `shared-100000.xlsx` and
`shared-1000000.xlsx`, three runs each, in the same container and with
the same yardstick as the table, and checks peak memory, never time (a
run that fails or reads the wrong number of rows fails it too). For each
of `read` and `verify`:

1. The peak on `shared-1000000.xlsx` is at most **64 MiB**.
2. The peak on `shared-1000000.xlsx` is at most the peak on
   `shared-100000.xlsx` plus **32 MiB**, the size the shared strings
   table may reach before it moves to a temporary file. Ten times the rows
   may not cost ten times the memory.

How the limits were chosen:

- 64 MiB: the larger of the CLI's `read` on `shared-1000000.xlsx` here
  (35 MiB) and on CI (36 MiB), times 1.5, rounded up to a multiple of
  16 MiB. The 1.5 absorbs what moves between machines (glibc's malloc
  arenas follow the core count).
- 32 MiB: the default `sharedStringsSpillBytes`. Today the growth is
  22 MiB for `read` and none for `verify`.

The gate does not cover the Ruby SDK or the other SDKs (the value
conversion of each language), the gems, or time. A limit changes only
with this page: measure the table again, and give the reason in the pull
request that changes it.

## Reproducing

All three run in Docker; none needs Ruby or Rust on the host.

1. `make fixtures:generate` writes the four workbooks (about 15 seconds;
   the targets below run it first themselves).
2. `make bench:compare` measures every reader on every workbook and
   writes `dist/bench/compare.md` (the tables above) and
   `dist/bench/compare.json` (every run). It takes about half an hour,
   most of it the gems on 1,000,000 rows. It is not a gate: its figures
   change with the machine and the day.
3. `make bench:memory` runs the gate: one PASS or FAIL line, the figures
   in `dist/bench/memory.md` and, without `V=1`, `.make-logs/bench_memory.log`.

`make bench:cli` measures the CLI alone on the four workbooks, for a
machine where the gems are not wanted.
