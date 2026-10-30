# frozen_string_literal: true

module XlsxGraft
  # A cell's time of day, with no date: the workbook's wall-clock time, such
  # as 13:45:30.250. Ruby has no type for one, and a {Time} would need a date
  # the workbook never gave. An elapsed time is not one of these: it is a
  # Float of seconds.
  TimeOfDay = Data.define(:hour, :minute, :second, :millisecond)
end
