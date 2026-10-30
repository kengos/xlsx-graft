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
