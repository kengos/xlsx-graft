# frozen_string_literal: true

module XlsxGraft
  # One cell that holds a value, in {GridRow#cells}.
  #
  # `ref` is the cell's A1 reference. `type` is "string", "number",
  # "boolean", "date", "time", "datetime", "elapsed" or "error". `value` is
  # the JSON value of `xlsx-graft read`, unchanged: a String for a string and
  # an error (`"#N/A"`), a Float for a number, true or false for a boolean,
  # an Integer of milliseconds for an elapsed time, and a String
  # (`"2024-01-15"`, `"08:30:00.000"`, `"2024-01-15T08:30:00.000"`) for a
  # date, a time and a date-time. The text of a string or an error is tidied
  # for display: control characters other than tab, LF and CR are removed, so
  # are characters that reorder or hide a line, and it is cut at 1,000
  # characters. `truncated` is true when it was cut, so `value` is not the
  # whole text.
  GridCell = Data.define(:ref, :type, :value, :truncated)
end
