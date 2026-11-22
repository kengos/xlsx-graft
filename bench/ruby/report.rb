# frozen_string_literal: true

# The report of a compare.rb run, from the Hash it collected:
#
#   { "conditions" => {...}, "runs" => 3, "timeout_seconds" => 1800,
#     "memory_limit_gib" => 8, "results" => [ { "book" => "rows-100000",
#       "reader" => "rubyxl", "status" => "ok", "exit" => nil,
#       "rss_max_kib" => 421888, "seconds_median" => 12.3, "runs" => [...] } ] }
#
# "exit" is nil when every run of the pair is ok, else the exit status of the
# first run that is not.
#
# A result of a loading-only run has "book" => "startup" and a startup name
# (Plan::STARTUPS) as its "reader". `json` writes the Hash as it is; `markdown`
# writes the conditions, then a table of the readings (a row for each reader, a
# column for each book) and one of the loading alone. A reading's cell is its
# peak memory, in brackets what it adds to the reader's loading alone, and its
# median time: `412 MiB (+380) · 12.3 s`.

require "json"
require_relative "plan"

module Bench
  module Report
    STARTUP = "startup"

    module_function

    def json(data)
      "#{JSON.pretty_generate(data)}\n"
    end

    def markdown(data)
      results = data.fetch("results")
      books = results.map { |r| r["book"] }.uniq - [STARTUP]
      readers = results.reject { |r| r["book"] == STARTUP }.map { |r| r["reader"] }.uniq
      [
        "# xlsx-graft benchmark\n",
        conditions(data["conditions"]),
        "## Reading and verifying\n",
        "Peak memory (in brackets: what it adds to the reader's loading alone) and median time of " \
        "#{data["runs"]} runs, each allowed #{duration(data["timeout_seconds"])} " \
        "and #{data["memory_limit_gib"]} GiB.#{output_note(readers)}\n",
        readings(data, books, readers),
        "## Loading alone\n",
        "Peak memory and median time of loading the reader's library (the CLI: `--version`).\n",
        loading(data)
      ].join("\n")
    end

    # What the readings print, as a sentence; the Ruby readers' part only when a
    # Ruby reader is in the table.
    def output_note(readers)
      note = " The CLI's read writes every row as a JSON line to a file"
      return "#{note}." unless readers.any? { |r| !r.start_with?("cli_") }

      "#{note}; the Ruby readers print only the counts."
    end

    # Minutes when the seconds are a whole number of minutes, else seconds.
    def duration(seconds)
      (seconds % 60).zero? ? "#{seconds / 60} min" : "#{seconds} s"
    end

    def conditions(conditions)
      lines = conditions.map { |name, value| "- #{name}: #{value}" }
      "## Conditions\n\n#{lines.join("\n")}\n"
    end

    def readings(data, books, readers)
      results = data.fetch("results")
      startups = results.select { |r| r["book"] == STARTUP }
      header = ["Reader", *books]
      rows = readers.map do |reader|
        cells = books.map do |book|
          result = results.find { |r| r["book"] == book && r["reader"] == reader }
          result ? cell(data, result, startup_for(startups, reader)) : "-"
        end
        [Plan.label(reader), *cells]
      end
      "#{table(header, rows)}\n"
    end

    def loading(data)
      startups = data.fetch("results").select { |r| r["book"] == STARTUP }
      rows = startups.map { |s| [Plan::STARTUPS.fetch(s["reader"]), cell(data, s, nil)] }
      "#{table(["Library", "Loading"], rows)}\n"
    end

    # The startup result of the library `reader` loads, if one was measured.
    def startup_for(startups, reader)
      startups.find { |s| s["reader"] == Plan.startup_of(reader) }
    end

    def cell(data, result, startup)
      case result["status"]
      when "timeout" then "timed out (#{duration(data["timeout_seconds"])})"
      when "oom" then "killed past #{data["memory_limit_gib"]} GiB"
      when "mismatch" then "wrong values"
      when "failed" then failed(result)
      when "ok" then measured(result, startup)
      else result["status"].to_s
      end
    end

    # A failed pair's exit status; a run that exited 0 without a GNU time report
    # has the reason in its runs.
    def failed(result)
      return "failed (exit #{result["exit"]})" unless result["exit"].nil? || result["exit"].zero?

      "failed (#{result["runs"]&.find { |r| r["report"] }&.fetch("report", nil) || "exit 0"})"
    end

    def measured(result, startup)
      return "-" if result["rss_max_kib"].nil? || result["seconds_median"].nil?

      mib = (result["rss_max_kib"] / 1024.0).round
      base = startup && startup["status"] == "ok" ? startup["rss_max_kib"] : nil
      added = base ? " (+#{mib - (base / 1024.0).round})" : ""
      format("%<mib>d MiB%<added>s · %<seconds>.1f s", mib: mib, added: added, seconds: result["seconds_median"])
    end

    def table(header, rows)
      lines = [header, header.map { "---" }, *rows].map { |row| "| #{row.join(" | ")} |" }
      lines.join("\n")
    end
  end
end
