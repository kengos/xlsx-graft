# frozen_string_literal: true

module XlsxGraft
  # One row of {Grid#rows}: its number from 1 as `row`, and its {GridCell}s
  # from the left as `cells`. Only cells that hold a value are listed.
  GridRow = Data.define(:row, :cells)
end
