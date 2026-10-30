# frozen_string_literal: true

module XlsxGraft
  # A cell holding an error value, its text as the workbook writes it
  # ("#N/A", "#DIV/0!"). Kept apart from a String so that a cell holding the
  # text "#N/A" is not read as the error.
  CellError = Data.define(:text)
end
