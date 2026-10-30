# frozen_string_literal: true

module XlsxGraft
  # The root of what this gem raises. A call into the engine never raises; it
  # returns a {Result}. What raises is a setup that cannot reach the engine,
  # or a book used after its block. A call made wrongly raises Ruby's own
  # ArgumentError.
  class Error < StandardError; end

  # No engine library could be found or loaded. A configuration mistake, not a
  # failed call, so it is raised rather than returned as a {Result}.
  class LibraryNotFound < Error; end

  # A {Book} or {Sheet} used after the block of {XlsxReader.open} ended,
  # which closed it.
  class ClosedError < Error; end
end
