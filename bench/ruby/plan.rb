# frozen_string_literal: true

# What compare.rb measures: the books, the readers, the command each pair runs,
# and what its output must say for the run to count.
#
# A book is named <kind>-<rows>, as `make fixtures:generate` writes it
# (fixtures/generated/<name>.xlsx): "rows" has 5 columns of values in every
# row, "shared" 3, and each has one header row, so <rows> + 1 rows hold
# 5 * (<rows> + 1) and 3 * (<rows> + 1) values.

require "json"

module Bench
  module Plan
    BOOKS = %w[rows-100000 rows-1000000 shared-100000 shared-1000000].freeze
    COLUMNS = { "rows" => 5, "shared" => 3 }.freeze

    READ_SCRIPT = File.join(__dir__, "read.rb")

    # In the order of the report's rows. `startup` names the reader whose
    # loading cost is subtracted; `label` is the report's name for the row.
    RUBY_READERS = {
      "rubyxl" => ["rubyXL", "rubyxl"],
      "roo" => ["Roo", "roo"],
      "creek" => ["Creek", "creek"],
      "xsv" => ["Xsv", "xsv"],
      "simple_xlsx_reader" => ["simple_xlsx_reader", "simple_xlsx_reader"],
      "xlsx_graft" => ["xlsx-graft Ruby SDK, read", "xlsx_graft"],
      "xlsx_graft_verify" => ["xlsx-graft Ruby SDK, verify", "xlsx_graft"]
    }.freeze
    CLI_READERS = {
      "cli_read" => ["xlsx-graft CLI, read", "cli"],
      "cli_verify" => ["xlsx-graft CLI, verify", "cli"]
    }.freeze
    READERS = RUBY_READERS.merge(CLI_READERS).freeze

    # The loading-only runs, one per library: the key is the startup name.
    STARTUPS = {
      "rubyxl" => "rubyXL", "roo" => "Roo", "creek" => "Creek", "xsv" => "Xsv",
      "simple_xlsx_reader" => "simple_xlsx_reader",
      "xlsx_graft" => "xlsx-graft Ruby SDK", "cli" => "xlsx-graft CLI"
    }.freeze

    module_function

    # The CLI's path, from XLSX_GRAFT_CLI (mk/bench.mk sets it from CLI_LOCAL,
    # so that the path is written in one place).
    def cli
      ENV.fetch("XLSX_GRAFT_CLI") { abort "XLSX_GRAFT_CLI is not set: it names the xlsx-graft CLI that cli_read, cli_verify and the run that measures the CLI's loading start" }
    end

    def label(reader)
      READERS.fetch(reader).first
    end

    def startup_of(reader)
      READERS.fetch(reader).last
    end

    def book_path(dir, book)
      File.join(dir, "#{book}.xlsx")
    end

    # [rows, values] the book must give, or nil for a name that is not a book's.
    def expected_counts(book)
      kind, rows = book.match(/\A(rows|shared)-(\d+)\z/)&.captures
      return nil if kind.nil?

      [rows.to_i + 1, COLUMNS.fetch(kind) * (rows.to_i + 1)]
    end

    # The command of one run: a reading of `path`, or the loading alone when
    # `path` is nil (the startup name goes in `reader`).
    def command(reader, path)
      case reader
      when "cli" then [cli, "--version"]
      when "cli_read" then [cli, "--format", "json", "read", path]
      when "cli_verify" then [cli, "--format", "json", "verify", path]
      else [RbConfig.ruby, "-rbundler/setup", READ_SCRIPT, reader, path || "--require-only"]
      end
    end

    # A lambda from the last line a run printed to whether it says the book was
    # read, or nil when nothing needs checking (loading alone).
    def check(reader, book)
      rows, cells = book && expected_counts(book)
      case reader
      when "xlsx_graft_verify", "cli_verify" then ->(line) { json(line)["ok"] == true }
      when "cli_read" then ->(line) { json(line).values_at("ok", "rows") == [true, rows] }
      when "xlsx_graft" then ->(line) { json(line).values_at("ok", "rows", "cells") == [true, rows, cells] }
      else
        ->(line) { json(line).values_at("rows", "cells") == [rows, cells] }
      end
    end

    # The parsed line, or an empty Hash when it is not JSON.
    def json(line)
      parsed = JSON.parse(line.to_s)
      parsed.is_a?(Hash) ? parsed : {}
    rescue JSON::ParserError
      {}
    end
  end
end
