# frozen_string_literal: true

require "json"

module XlsxGraft
  # Turns what a verify function returned into a {Result}: its status and,
  # on OK, the result's JSON
  # `{"ok", "failOn", "counts", "error", "diagnostics"}`.
  class VerifyReport
    class << self
      # @param status [Integer]
      # @param json [String, nil]
      # @return [Result] successful when `ok`, with no value. `counts` and
      #   `failOn` are not kept: the counts are those of `diagnostics`
      #   by severity.
      # @raise [Error] for XLSX_GRAFT_ERR_INVALID_ARG, which only a call this
      #   gem built wrongly gets
      def call(status, json)
        return parsed(json) if status == StatusDiagnostic::OK
        raise Error, bug(status) if status == StatusDiagnostic::INVALID_ARG

        Result.failure([], error: StatusDiagnostic.call(status))
      end

      private

      def bug(status)
        "the engine refused a call this gem built (status #{status}): a bug of the gem"
      end

      def parsed(json)
        report = JSON.parse(json, freeze: true)
        diagnostics = report.fetch("diagnostics").map { |r| EngineDiagnostics.diagnostic(r) }.freeze
        return Result.success(nil, diagnostics:) if report.fetch("ok") == true

        Result.failure(diagnostics, error: failure(report.fetch("error")))
      rescue JSON::ParserError, KeyError, TypeError, NoMethodError => e
        unreadable("the engine's result could not be read: #{e.message}")
      end

      def failure(error)
        error && StatusDiagnostic.of(error.fetch("code"), error.fetch("message"))
      end

      def unreadable(message)
        Result.failure([], error: StatusDiagnostic.of("diagnostics_unreadable", message))
      end
    end
  end
end
