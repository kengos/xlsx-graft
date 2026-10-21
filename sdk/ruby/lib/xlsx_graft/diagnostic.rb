# frozen_string_literal: true

module XlsxGraft
  # One thing the engine reported about a call. `code` is the stable,
  # machine-readable part; `message` is for people.
  Diagnostic = Data.define(:code, :message)
end
