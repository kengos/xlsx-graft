# frozen_string_literal: true

require "json"

module XlsxGraft
  # Builds a {Result} from the status and the text a view function returned.
  # On OK the text is the view's JSON `{"ok", "error", "diagnostics", ...}`
  # and then the keys of what the view shows.
  class ViewReport
    class << self
      # @param status [Integer]
      # @param text [String, nil] the view's JSON for OK, the message for
      #   XLSX_GRAFT_ERR_REQUEST_INVALID (nil or empty when it was not read)
      # @param content [#call] {TreeContent} or {GridContent}, which reads the
      #   keys of what the view shows into its value
      # @return [Result] successful when the engine's `ok` is true. It holds
      #   the value (a {Tree} or a {Grid}) whether `ok` is true or not; the
      #   value is nil only when this gem failed to get one (a status other
      #   than OK, or JSON it cannot read).
      # @raise [ArgumentError] for XLSX_GRAFT_ERR_REQUEST_INVALID, with the
      #   engine's message
      # @raise [Error] for XLSX_GRAFT_ERR_INVALID_ARG, which only a call this
      #   gem built wrongly gets
      def call(status, text, content)
        return parsed(text, content) if status == StatusDiagnostic::OK
        raise ArgumentError, refusal(text) if status == StatusDiagnostic::REQUEST_INVALID
        raise Error, bug(status) if status == StatusDiagnostic::INVALID_ARG

        Result.failure([], error: StatusDiagnostic.call(status))
      end

      private

      # The engine's message, or a fixed one when it could not be read (under
      # Ruby's M:N threads the message may be another call's or empty).
      def refusal(text)
        return text unless text.nil? || text.empty?

        "the engine refused a value of the options (status #{StatusDiagnostic::REQUEST_INVALID}) " \
          "and gave no reason"
      end

      def bug(status)
        "the engine refused a call this gem built (status #{status}): a bug of the gem"
      end

      def parsed(json, content)
        view = JSON.parse(json, freeze: true)
        value = content.call(view)
        diagnostics = view.fetch("diagnostics").map { |r| EngineDiagnostics.diagnostic(r) }.freeze
        return Result.success(value, diagnostics:) if view.fetch("ok") == true

        Result.failure(diagnostics, error: failure(view.fetch("error")), value:)
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
