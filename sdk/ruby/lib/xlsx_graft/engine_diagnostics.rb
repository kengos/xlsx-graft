# frozen_string_literal: true

require "json"

module XlsxGraft
  # Reads the JSON `xlsx_graft_book_diagnostics` writes,
  # `{"has_errors": B, "error": E, "diagnostics": [...]}`.
  class EngineDiagnostics
    # What the book said: whether a diagnostic is an error, its first
    # failure (a {Diagnostic}, or nil), and its {Diagnostic}s.
    Report = Data.define(:has_errors, :error, :diagnostics)

    class << self
      # @param json [String]
      # @return [Report]
      # @raise [JSON::ParserError, KeyError] for JSON not of that shape
      def call(json)
        report = JSON.parse(json, freeze: true)
        error = report.fetch("error")
        Report.new(
          has_errors: report.fetch("has_errors"),
          error: error && StatusDiagnostic.of(error.fetch("code"), error.fetch("message")),
          diagnostics: report.fetch("diagnostics").map { |record| of(record) }.freeze
        )
      end

      private

      def of(record)
        Diagnostic.new(
          code: record.fetch("code"), severity: record.fetch("severity"),
          message: record.fetch("message"), args: record.fetch("args"),
          location: record.fetch("location"), decided_by: record.fetch("decidedBy"),
          fix: record.fetch("fix")
        )
      end
    end
  end
end
