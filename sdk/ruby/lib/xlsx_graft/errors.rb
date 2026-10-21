# frozen_string_literal: true

module XlsxGraft
  # The root of what this gem raises. A call into the engine never raises; it
  # returns a {Result}. What raises is a setup that cannot reach the engine.
  class Error < StandardError; end

  # No engine library could be found or loaded. A configuration mistake, not a
  # failed call, so it is raised rather than returned as a {Result}.
  class LibraryNotFound < Error; end
end
