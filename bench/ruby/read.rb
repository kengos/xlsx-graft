# frozen_string_literal: true

# One measured run: read the first sheet of a workbook with one reader and print
# what was read as one line of JSON, so compare.rb can tell a reader that read
# the book from one that read something else.
#
#   ruby read.rb <reader> <workbook.xlsx>
#   ruby read.rb <reader> --require-only   (not xlsx_graft_verify)
#
# Readers: rubyxl, roo, creek, xsv, simple_xlsx_reader, xlsx_graft (the Ruby
# SDK's XlsxReader) and xlsx_graft_verify (its Verifier). A reader prints
# {"rows":R,"cells":C}: R the rows it yields, C the cells in them that hold a
# value (not nil); the Ruby SDK's reader adds "ok", whether its Result is a
# success. The verifier prints {"ok":true|false}. --require-only loads the
# reader's library (the Ruby SDK's: the engine library as well, which
# XlsxReader.open and Verifier.new would load) and prints nothing: the cost of
# starting the reader, which compare.rb subtracts from a reading's.
#
# Each reader touches every value, as a program that uses the sheet would, and
# keeps none: a reader that streams must not have its memory include rows that
# read.rb keeps.
#
# The Ruby SDK is loaded from the checkout (/repo/sdk/ruby/lib) and finds the
# engine library through XLSX_GRAFT_LIBRARY.

require "json"

SDK_LIB = File.expand_path("../../sdk/ruby/lib", __dir__)

# Counts the rows and the non-nil values of the sheet, from a block that yields
# each row's values (an Array, or nothing for a row that is not there).
class Tally
  attr_reader :rows, :cells
  attr_writer :ok

  def initialize
    @rows = 0
    @cells = 0
    @ok = nil
  end

  def row(values)
    return if values.nil?

    @rows += 1
    values.each { |value| @cells += 1 unless value.nil? }
  end

  def to_json(*)
    JSON.generate({ ok: @ok, rows: @rows, cells: @cells }.compact)
  end
end

# Each reader: the library to load, and how to read the sheet into a Tally.
READERS = {
  "rubyxl" => {
    load: -> { require "rubyXL" },
    read: lambda do |path, tally|
      RubyXL::Parser.parse(path).worksheets[0].each do |row|
        tally.row(row&.cells&.map { |cell| cell&.value })
      end
    end
  },
  "roo" => {
    load: -> { require "roo" },
    read: lambda do |path, tally|
      Roo::Excelx.new(path).each_row_streaming(pad_cells: false) do |row|
        tally.row(row.map(&:value))
      end
    end
  },
  "creek" => {
    load: -> { require "creek" },
    read: lambda do |path, tally|
      Creek::Book.new(path).sheets[0].simple_rows.each { |row| tally.row(row.values) }
    end
  },
  "xsv" => {
    load: -> { require "xsv" },
    read: lambda do |path, tally|
      Xsv.open(path).sheets[0].each { |row| tally.row(row) }
    end
  },
  "simple_xlsx_reader" => {
    load: -> { require "simple_xlsx_reader" },
    read: lambda do |path, tally|
      SimpleXlsxReader.open(path).sheets.first.rows.each { |row| tally.row(row) }
    end
  },
  "xlsx_graft" => {
    load: lambda do
      $LOAD_PATH.unshift(SDK_LIB)
      require "xlsx_graft"
    end,
    read: lambda do |path, tally|
      result = XlsxGraft::XlsxReader.open(path) do |book|
        book.sheet.each_row { |row| tally.row(row.values) }
      end
      tally.ok = result.success?
    end
  }
}.freeze

VERIFIER = {
  load: READERS.fetch("xlsx_graft").fetch(:load),
  read: ->(path) { JSON.generate(ok: XlsxGraft::Verifier.new.verify(path).success?) }
}.freeze

reader, target = ARGV
abort "usage: read.rb <reader> <workbook.xlsx>|--require-only" if reader.nil? || target.nil?

if reader == "xlsx_graft_verify"
  VERIFIER.fetch(:load).call
  puts VERIFIER.fetch(:read).call(target)
else
  spec = READERS.fetch(reader) { abort "unknown reader #{reader.inspect}" }
  spec.fetch(:load).call
  XlsxGraft::Library.open if reader == "xlsx_graft" && target == "--require-only"
  unless target == "--require-only"
    tally = Tally.new
    spec.fetch(:read).call(target, tally)
    puts tally.to_json
  end
end
