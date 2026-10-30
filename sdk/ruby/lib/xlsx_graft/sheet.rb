# frozen_string_literal: true

module XlsxGraft
  # One sheet of a {Book}, asked for by name or index, whose rows
  # {#each_row} reads.
  class Sheet
    # The largest index a size_t holds.
    MAX_INDEX = (2**(8 * Fiddle::SIZEOF_SIZE_T)) - 1

    # @api private: {Book#sheet} makes it
    # @raise [ArgumentError] for neither a String nor an Integer from 0
    def initialize(book, which)
      raise ArgumentError, "a sheet is a name or an index from 0, not #{which.inspect}" unless
        which.is_a?(String) || (which.is_a?(Integer) && which.between?(0, MAX_INDEX))

      @book = book
      @which = which
    end

    # Yields each row the sheet writes, as a {Row}, in order, reading one at
    # a time: no more of the sheet is held than the row yielded. Only the
    # rows the sheet writes come. Each call reads the sheet from its start.
    #
    # Reading stops at the first problem in the sheet, the rows before it
    # having come, and yields nothing when the engine refuses the sheet (no
    # such sheet, not a worksheet); either way the {Result} of
    # {XlsxReader.open} fails, saying why. An Enumerator read from outside
    # (`next`) holds its rows until it is read to the end or the book's
    # block ends.
    #
    # @yieldparam row [Row]
    # @return [self, Enumerator] an Enumerator of the rows when no block is
    #   given
    # @raise [ClosedError] after the book's block ended
    def each_row(&)
      return enum_for(:each_row) unless block_given?

      rows = @book.open_rows(@which)
      read(rows, &) if rows
      self
    end

    private

    def read(rows)
      while (row = @book.next_row(rows))
        yield row
      end
    ensure
      @book.close_rows(rows)
    end
  end
end
