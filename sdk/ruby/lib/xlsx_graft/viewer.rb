# frozen_string_literal: true

module XlsxGraft
  # Shows a workbook's parts and a sheet's cells through the C ABI: the
  # engine's `tree` and `describe_sheet` views (docs/sdk.md § Views in an SDK).
  # The limits a workbook is read under are the CLI's defaults; no policy is
  # taken.
  #
  # A viewer does not change once it is made, so one may be shared by any
  # number of threads, and their calls may run at once.
  #
  # @example
  #   viewer = XlsxGraft::Viewer.new
  #   tree = viewer.tree("book.xlsx").value
  #   tree.parts.each { |part| puts "#{part.name} #{part.size}" }
  #   result = viewer.describe_sheet("book.xlsx", sheet: "Data", range: "A1:H50", max_rows: 20)
  #   result.value.rows.each { |row| p row.cells.map(&:value) } if result.success?
  class Viewer
    # @param library [String, nil] the engine library's path, used when
    #   XLSX_GRAFT_LIBRARY is not set
    # @raise [LibraryNotFound] when no engine library is found
    def initialize(library: nil)
      @call = ViewCall.new(Library.open(path: library))
      freeze
    end

    # The parts of the workbook at `path` and the relationships between them.
    #
    # @param path [String, #to_path]
    # @return [Result] successful when the engine's `ok` is true. Its value is
    #   a {Tree} also when it is not (a workbook that is broken, or a file that
    #   cannot be read, gives a Tree with the parts that were read, perhaps
    #   none); it is nil only when this gem failed to get one (`panic`,
    #   `null_arg`, `unknown_status`, `diagnostics_unreadable`, or `io` for a
    #   path that is not UTF-8 whose file this gem cannot read). A file that
    #   cannot be opened is an `io` `error`.
    def tree(path)
      PathInput.call(path) { |how, input| run(:"tree_#{how}", input, "", TreeContent) }
    end

    # As {#tree}, for the workbook in the bytes of `data`, which this gem
    # copies for the call.
    #
    # @param data [String]
    # @return [Result] (see #tree)
    def tree_bytes(data)
      run(:tree_bytes, data, "", TreeContent)
    end

    # The cells of one sheet of the workbook at `path`.
    #
    # @param path [String, #to_path]
    # @param sheet [Integer, String, Symbol, nil] the sheet's index (an
    #   Integer) or name; nil is the first sheet
    # @param range [String, nil] an A1 range such as `"A1:H50"`: the cells
    #   shown start at its top left cell (A1 without it) and never go past it
    # @param max_rows [Integer, nil] 1 to 10000 (100 without it)
    # @param max_columns [Integer, nil] 1 to 1000 (30 without it)
    # @return [Result] successful when the engine's `ok` is true. Its value is
    #   a {Grid} also when it is not (a sheet that is not found gives a Grid
    #   whose `sheet` is nil); it is nil in the cases of {#tree}.
    # @raise [ArgumentError] when the engine refuses a value (`range` that is
    #   no A1 range, `max_rows` out of range, a `sheet` that is neither a
    #   name nor an index), with the engine's message; or before the call
    #   when an Integer is beyond what a double holds, a value cannot be
    #   written as JSON, or the options are more than 2 MiB
    def describe_sheet(path, sheet: nil, range: nil, max_rows: nil, max_columns: nil)
      options = ViewOptions.call(sheet:, range:, max_rows:, max_columns:)
      PathInput.call(path) do |how, input|
        run(:"describe_sheet_#{how}", input, options, GridContent)
      end
    end

    # As {#describe_sheet}, for the workbook in the bytes of `data`, which
    # this gem copies for the call.
    #
    # @param data [String]
    # @return [Result] (see #describe_sheet)
    def describe_sheet_bytes(data, sheet: nil, range: nil, max_rows: nil, max_columns: nil)
      options = ViewOptions.call(sheet:, range:, max_rows:, max_columns:)
      run(:describe_sheet_bytes, data, options, GridContent)
    end

    private

    def run(name, input, options, content)
      ViewReport.call(*@call.call(name, input, options), content)
    end
  end
end
