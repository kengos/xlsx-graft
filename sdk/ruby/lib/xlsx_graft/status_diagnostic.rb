# frozen_string_literal: true

module XlsxGraft
  # Turns a C ABI status other than XLSX_GRAFT_OK into a {Diagnostic}, for
  # a status the engine keeps no failure of: one with no book to keep it
  # (an open), or after which its diagnostics cannot be read (a panic).
  #
  # The statuses that refuse a sheet are named as `xlsx-graft read` names
  # them in its end line's `error.code`; the book's diagnostics say them in
  # full, naming the sheet. The others name the status codes of
  # engine/capi/include/xlsx_graft.h.
  class StatusDiagnostic
    OK = 0
    PANIC = 2
    IO = 3
    WORKBOOK_UNREADABLE = 5
    SHEET_UNREADABLE = 9

    CODES = {
      1 => ["null_arg", "the engine was passed a null pointer"],
      PANIC => ["panic", "the engine panicked; the call did not complete"],
      IO => ["io", "the host failed to open, read or write a file"],
      4 => ["invalid_arg", "the engine was passed text that is not UTF-8"],
      WORKBOOK_UNREADABLE => ["workbook_unreadable",
                              "the workbook cannot be read; the diagnostics say why"],
      6 => ["sheet_not_found", "no such sheet"],
      7 => ["sheet_ambiguous", "more than one sheet has the name, letter case aside"],
      8 => ["sheet_not_worksheet", "the sheet is not a worksheet"],
      SHEET_UNREADABLE => ["sheet_unreadable", "the sheet cannot be read; the diagnostics say why"],
      10 => ["closed", "the rows' book was closed"]
    }.freeze

    # The args and location of a diagnostic the gem makes: it has neither.
    NONE = {}.freeze

    class << self
      # @param status [Integer] the status a C ABI function returned
      # @param message [String, nil] what to say in place of the status's own
      #   message, for one that knows more (why the host failed)
      # @return [Diagnostic]
      def call(status, message: nil)
        code, default = CODES.fetch(status) do
          ["unknown_status", "the engine returned status #{status}, which this gem does not know"]
        end
        of(code, message || default)
      end

      # An "error" diagnostic of the gem's own, not from the engine's
      # diagnostics.
      #
      # @return [Diagnostic]
      def of(code, message)
        Diagnostic.new(code:, severity: "error", message:, args: NONE, location: NONE,
                       decided_by: nil, fix: nil)
      end
    end
  end
end
