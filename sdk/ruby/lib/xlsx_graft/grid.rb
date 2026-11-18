# frozen_string_literal: true

module XlsxGraft
  # The cells of one sheet, as {Viewer#describe_sheet} returns them. Each
  # attribute is the key of the same name in the engine's `describe_sheet`
  # JSON (docs/sdk.md § Describing a sheet).
  #
  # `sheet` is the {SheetInfo} of the sheet that was read, nil when no sheet
  # was chosen (the workbook could not be read, no sheet is the one asked
  # for, or two sheets have the requested name). `dimension` is the sheet's
  # own `dimension` as written (a value for display, not checked to be a
  # range), nil when it has none. `used` is the range of the cells that hold
  # a value in the whole sheet, nil when no cell holds a value or the sheet
  # could not be read. `shown` is the range the view looked at:
  # from the top left corner of the requested range (A1 without one), at most
  # the requested rows and columns, never past the range or past `used`; nil
  # when that is nothing. Every range has two corners, `A1:A1` for one cell.
  # `truncated` is a {GridTruncated}. `complete` is whether the sheet was read
  # to its end with nothing stopping it; the result's diagnostics say what
  # stopped it. `rows` are the {GridRow}s of `shown` that have a cell with a
  # value, fewer than that when the view's caps (100,000 cells, 1,000,000
  # characters of string and error text) left rows out, which `truncated.rows`
  # says. `merges` are the merged ranges that overlap `shown`.
  Grid = Data.define(:sheet, :dimension, :used, :shown, :truncated, :complete, :rows, :merges)
end
