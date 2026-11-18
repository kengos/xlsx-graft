# frozen_string_literal: true

module XlsxGraft
  # Whether {Grid#rows} and {Grid#merges} leave something out. `rows` is true
  # when cells with a value in the requested range lie below `shown`, or when
  # the view's caps (100,000 cells, 1,000,000 characters of string and error
  # text) left out rows of `shown`. `columns` is true when cells with a value
  # in the requested range lie to the right of the last column of `shown`, in
  # any row, below `shown` too. `merges` is true when
  # merged ranges were left out.
  GridTruncated = Data.define(:rows, :columns, :merges)
end
