# frozen_string_literal: true

# The check of `make bench:memory`: reads the JSON compare.rb wrote (the shape
# is in report.rb's header) and checks the CLI's peak memory against two
# promises, for each of cli_read and cli_verify:
#
# 1. the peak on shared-1000000 is at most --peak-mib MiB;
# 2. the peak on shared-1000000 is at most the peak on shared-100000 plus
#    --growth-mib MiB.
#
#   ruby memory.rb <compare.json> --peak-mib N --growth-mib M
#
# A reading that is missing, whose status is not "ok" or that has no
# rss_max_kib fails the check, naming the book, the reader and the status.
# One line per reader goes to stdout (MiB rounded for display, the comparisons
# in KiB). Exits 1 when any check fails, else 0.

require "json"

module Bench
  module Memory
    READERS = %w[cli_read cli_verify].freeze
    SMALL = "shared-100000"
    LARGE = "shared-1000000"
    USAGE = "usage: memory.rb <compare.json> --peak-mib N --growth-mib M"

    module_function

    def parse(argv)
      path, *rest = argv
      abort USAGE unless path && rest.size == 4 && rest.each_slice(2).to_h.keys.sort == %w[--growth-mib --peak-mib]
      limits = rest.each_slice(2).to_h
      values = limits.values_at("--peak-mib", "--growth-mib")
      abort USAGE unless values.all? { |v| v.match?(/\A\d+\z/) }
      [path, *values.map { |v| Integer(v, 10) }]
    end

    def reading(results, book, reader)
      results.find { |r| r["book"] == book && r["reader"] == reader }
    end

    def mib(kib)
      (kib / 1024.0).round
    end

    # Why a reading has no figure to check.
    def why(result)
      return "missing" if result.nil?
      return "status #{result["status"].inspect}" if result["status"] != "ok"

      "no rss_max_kib"
    end

    # The line for a reader and whether it passed.
    def check(results, reader, peak_mib, growth_mib)
      small = reading(results, SMALL, reader)
      large = reading(results, LARGE, reader)
      bad = [[SMALL, small], [LARGE, large]].find { |_, r| r.nil? || r["status"] != "ok" || r["rss_max_kib"].nil? }
      return ["#{reader}: #{bad[0]} #{why(bad[1])}: FAIL", false] if bad

      small_kib = small["rss_max_kib"]
      large_kib = large["rss_max_kib"]
      peak_ok = large_kib <= peak_mib * 1024
      growth_ok = large_kib <= small_kib + growth_mib * 1024
      line = "#{reader}: #{SMALL} #{mib(small_kib)} MiB, #{LARGE} #{mib(large_kib)} MiB; " \
             "peak #{mib(large_kib)} <= #{peak_mib} MiB: #{peak_ok ? "ok" : "FAIL"}; " \
             "growth +#{mib(large_kib - small_kib)} <= +#{growth_mib} MiB: #{growth_ok ? "ok" : "FAIL"}"
      [line, peak_ok && growth_ok]
    end

    def run(argv)
      path, peak_mib, growth_mib = parse(argv)
      results = JSON.parse(File.read(path)).fetch("results")
      checks = READERS.map { |reader| check(results, reader, peak_mib, growth_mib) }
      checks.each { |line, _| puts line }
      exit(1) unless checks.all? { |_, ok| ok }
    end
  end
end

Bench::Memory.run(ARGV) if $PROGRAM_NAME == __FILE__
