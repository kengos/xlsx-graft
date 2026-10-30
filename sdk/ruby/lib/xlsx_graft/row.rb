# frozen_string_literal: true

module XlsxGraft
  # One row a sheet writes: its number, from 1, and the value of each column
  # from A to its last cell, `nil` where a column has none. The values are
  # those {CellValues} documents.
  Row = Data.define(:number, :values)
end
