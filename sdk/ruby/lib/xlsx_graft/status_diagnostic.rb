# frozen_string_literal: true

module XlsxGraft
  # Turns a C ABI status other than XLSX_GRAFT_OK into a {Diagnostic}.
  #
  # The codes are provisional: they name the status codes of
  # engine/capi/include/xlsx_graft.h until the engine reports diagnostics of
  # its own, which replace them.
  class StatusDiagnostic
    OK = 0

    CODES = {
      1 => ["null_arg", "the engine was passed a null pointer"],
      2 => ["panic", "the engine panicked; the call did not complete"]
    }.freeze

    class << self
      # @param status [Integer] the status a C ABI function returned
      # @return [Diagnostic]
      def call(status)
        code, message = CODES.fetch(status) do
          ["unknown_status", "the engine returned status #{status}, which this gem does not know"]
        end
        Diagnostic.new(code: code, message: message)
      end
    end
  end
end
