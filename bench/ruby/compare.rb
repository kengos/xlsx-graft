# frozen_string_literal: true

# Compares xlsx-graft with the Ruby xlsx gems on the generated workbooks, and
# writes what it measured:
#
#   ruby compare.rb [--only cli] --out <prefix>
#
# <prefix>.json holds the conditions and, for each book and reader, the runs
# (status, peak memory, time), the status of the pair, the largest peak memory
# and the median time; <prefix>.md the same as tables (report.rb).
# `make bench:compare` and `make bench:cli` run it in the image bench/ruby builds.
#
# Each pair of a book (fixtures/generated/) and a reader (plan.rb) runs RUNS
# times, each in a process of its own under `timeout` and GNU time
# (measure.rb). A pair that times out or is killed for its memory is not run
# again: the runs left would take as long. Loading alone is measured for each
# library too, so a reading's memory can be shown net of it. One line per run
# goes to stderr as it finishes, so that a run of many minutes prints progress.
#
# Options that narrow or move a run: `--books a,b` (bench:memory passes it) and
# `--readers x,y` narrow what runs (the loading-alone runs follow the readers),
# `--book-dir DIR` reads the books from DIR, `--runs N` and `--timeout SECONDS`
# replace the constants below.

require "fileutils"
require "optparse"
require "rbconfig"
require "time"

module Bench
  RUNS = 3
  TIMEOUT_SECONDS = 1800
  MEMORY_LIMIT_GIB = 8 # what `make` gives the container (mk/bench.mk), named in the report
end

require_relative "measure"
require_relative "plan"
require_relative "report"

module Bench
  # Runs the pairs and writes the files.
  class Compare
    GEMS = %w[rubyXL roo creek xsv simple_xlsx_reader fiddle nokogiri rubyzip].freeze

    def initialize(options)
      @options = options
      @readers = options[:readers] || (options[:only_cli] ? Plan::CLI_READERS : Plan::READERS).keys
      @books = options[:books] || Plan::BOOKS
    end

    def call
      results = startups + readings
      data = {
        "conditions" => conditions, "runs" => @options[:runs], "timeout_seconds" => @options[:timeout],
        "memory_limit_gib" => MEMORY_LIMIT_GIB, "results" => results
      }
      FileUtils.mkdir_p(File.dirname(@options[:out]))
      File.write("#{@options[:out]}.json", Report.json(data))
      File.write("#{@options[:out]}.md", Report.markdown(data))
      warn "wrote #{@options[:out]}.json and #{@options[:out]}.md"
    end

    private

    def startups
      @readers.map { |r| Plan.startup_of(r) }.uniq.map do |startup|
        measure(Report::STARTUP, startup, Plan.command(startup, nil), nil)
      end
    end

    def readings
      @books.product(@readers).map do |book, reader|
        path = Plan.book_path(@options[:book_dir], book)
        measure(book, reader, Plan.command(reader, path), Plan.check(reader, book))
      end
    end

    # The runs of one pair, and what they come to.
    def measure(book, reader, command, check)
      runs = []
      @options[:runs].times do |n|
        run = Measure.run(command, timeout: @options[:timeout], check: check)
        runs << run
        progress(book, reader, n + 1, run)
        break if %i[timeout oom].include?(run[:status])
      end
      summary(book, reader, runs)
    end

    # `rss_max_kib` and `seconds_median` are nil unless the pair is ok: a run
    # that did not finish has no figures of its own to show, and an earlier
    # run's must not stand for it. The runs keep their raw values.
    def summary(book, reader, runs)
      bad = runs.find { |r| r[:status] != :ok }
      ok = bad.nil?
      rss = ok ? runs.filter_map { |r| r[:rss_kib] }.max : nil
      times = ok ? runs.filter_map { |r| r[:seconds] }.sort : []
      {
        "book" => book, "reader" => reader, "status" => (bad || runs.first)[:status].to_s,
        "exit" => bad && bad[:exit], # nil when every run is ok
        "rss_max_kib" => rss,
        "seconds_median" => times.empty? ? nil : (times[(times.size - 1) / 2] + times[times.size / 2]) / 2,
        "runs" => runs.map { |r| r.transform_keys(&:to_s).transform_values { |v| v.is_a?(Symbol) ? v.to_s : v } }
      }
    end

    def progress(book, reader, number, run)
      mib = run[:rss_kib] ? format("%.0f MiB", run[:rss_kib] / 1024.0) : "-"
      seconds = run[:seconds] ? format("%.1f s", run[:seconds]) : "-"
      warn "#{book} #{reader} run #{number}/#{@options[:runs]}: #{mib}, #{seconds}, #{run[:status]}"
    end

    def conditions
      {
        "date (UTC)" => Time.now.utc.strftime("%Y-%m-%d %H:%M"),
        "commit" => ENV.fetch("BENCH_COMMIT", "unknown"),
        "host" => ENV.fetch("BENCH_HOST", "unknown"),
        "container" => "#{`uname -m`.strip}, nproc #{`nproc`.strip}, memory limit #{memory_max}",
        "Ruby" => RUBY_VERSION,
        "gems" => GEMS.map { |name| "#{name} #{Gem.loaded_specs[name]&.version || "?"}" }.join(", "),
        "engine" => engine_version
      }
    end

    # The container's cgroup memory limit (cgroup v2, then v1), as bytes or "max".
    def memory_max
      %w[/sys/fs/cgroup/memory.max /sys/fs/cgroup/memory/memory.limit_in_bytes]
        .find { |path| File.exist?(path) }&.then { |path| File.read(path).strip } || "unknown"
    end

    def engine_version
      path = ENV["XLSX_GRAFT_CLI"]
      path && File.executable?(path) ? `#{path} --version`.strip : "no CLI built"
    end
  end
end

options = { runs: Bench::RUNS, timeout: Bench::TIMEOUT_SECONDS, book_dir: "fixtures/generated" }
OptionParser.new do |parser|
  parser.banner = "usage: compare.rb [--only cli] --out <prefix>"
  parser.on("--only cli", %w[cli], "the CLI's pairs alone") { options[:only_cli] = true }
  parser.on("--out PREFIX", "write PREFIX.json and PREFIX.md") { |v| options[:out] = v }
  parser.on("--books LIST") { |v| options[:books] = v.split(",") }
  parser.on("--readers LIST") { |v| options[:readers] = v.split(",") }
  parser.on("--book-dir DIR") { |v| options[:book_dir] = v }
  parser.on("--runs N", Integer) { |v| options[:runs] = v }
  parser.on("--timeout SECONDS", Integer) { |v| options[:timeout] = v }
end.parse!
abort "--runs must be at least 1" if options[:runs] < 1
abort "--timeout must be at least 1 second" if options[:timeout] < 1
abort "--out is required" unless options[:out]
(options[:readers] || []).each { |r| abort "unknown reader #{r}" unless Bench::Plan::READERS.key?(r) }
(options[:books] || []).each { |b| abort "not a book name: #{b}" unless Bench::Plan.expected_counts(b) }

Bench::Compare.new(options).call
