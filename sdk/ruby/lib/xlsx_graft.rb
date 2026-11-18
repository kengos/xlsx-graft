# frozen_string_literal: true

require_relative "xlsx_graft/version"
require_relative "xlsx_graft/errors"
require_relative "xlsx_graft/diagnostic"
require_relative "xlsx_graft/result"
require_relative "xlsx_graft/status_diagnostic"
require_relative "xlsx_graft/library"
require_relative "xlsx_graft/version_call"
require_relative "xlsx_graft/capi"
require_relative "xlsx_graft/capi/input"
require_relative "xlsx_graft/time_of_day"
require_relative "xlsx_graft/cell_error"
require_relative "xlsx_graft/cell_values"
require_relative "xlsx_graft/engine_diagnostics"
require_relative "xlsx_graft/row"
require_relative "xlsx_graft/sheet_info"
require_relative "xlsx_graft/sheet"
require_relative "xlsx_graft/book"
require_relative "xlsx_graft/xlsx_reader"
require_relative "xlsx_graft/path_input"
require_relative "xlsx_graft/verify_call"
require_relative "xlsx_graft/verify_report"
require_relative "xlsx_graft/verifier"
require_relative "xlsx_graft/tree"
require_relative "xlsx_graft/tree_part"
require_relative "xlsx_graft/tree_relationship"
require_relative "xlsx_graft/tree_target"
require_relative "xlsx_graft/grid"
require_relative "xlsx_graft/grid_truncated"
require_relative "xlsx_graft/grid_row"
require_relative "xlsx_graft/grid_cell"
require_relative "xlsx_graft/tree_content"
require_relative "xlsx_graft/grid_content"
require_relative "xlsx_graft/view_options"
require_relative "xlsx_graft/view_call"
require_relative "xlsx_graft/view_report"
require_relative "xlsx_graft/viewer"

# Ruby bindings for xlsx-graft, an engine that reads, verifies, creates and
# edits xlsx workbooks.
#
# The engine is a shared C library loaded through fiddle; nothing here
# reimplements it. Calls return a {Result} rather than raising. What raises is
# a setup with no engine library in it ({LibraryNotFound}).
#
# @example
#   result = XlsxGraft.version
#   puts result.value if result.success?
#
# @example Reading a sheet's rows ({XlsxReader})
#   XlsxGraft::XlsxReader.open("book.xlsx") do |book|
#     book.sheet("Data").each_row { |row| p row.values }
#   end
#
# @example Verifying a workbook ({Verifier})
#   verifier = XlsxGraft::Verifier.new(policy: { max_entries: 20_000 })
#   result = verifier.verify("book.xlsx", targets: ["excel >= 2016"])
#   result.diagnostics.each { |d| warn "#{d.severity} #{d.code}: #{d.message}" }
#
# @example Viewing a workbook ({Viewer})
#   viewer = XlsxGraft::Viewer.new
#   viewer.tree("book.xlsx").value.parts.each { |part| puts part.name }
#   grid = viewer.describe_sheet("book.xlsx", sheet: "Data", range: "A1:H50").value
#   grid.rows.each { |row| p row.cells.map(&:value) }
module XlsxGraft
  class << self
    # The engine's version.
    #
    # @param library [String, nil] the engine library's path, used when
    #   XLSX_GRAFT_LIBRARY is not set
    # @return [Result] the version string on success
    # @raise [LibraryNotFound] when no engine library is found
    def version(library: nil)
      VersionCall.call(Library.open(path: library))
    end
  end
end
