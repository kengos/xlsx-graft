# frozen_string_literal: true

module XlsxGraft
  # A workbook open to read, handed to the block of {XlsxReader.open}: its
  # sheets, and the rows of each through {#sheet}.
  #
  # Usable inside that block only. When the block ends, the book is closed,
  # and with it every sheet's rows; using it then raises {ClosedError}.
  #
  # Nothing here leaves the block early. A sheet the engine refuses (no such
  # sheet, an unreadable workbook) reads no rows, and {XlsxReader.open}'s
  # {Result} fails, saying why, as it does when a problem stops a sheet's
  # rows or the host fails. A panic in the engine ends every sheet of the
  # book: none reads a row after it, and the book lists no sheets.
  #
  # A book and its rows share state the library does not lock, so every call
  # into it, closes included, is made under the book's own lock, and what it
  # lends is copied out before the lock is let go.
  class Book
    KINDS = %w[worksheet chartsheet dialogsheet macrosheet unknown].freeze
    STATES = %w[visible hidden veryHidden].freeze
    CLOSED = "the book is closed: use it inside XlsxReader.open's block"

    # What a book whose diagnostics could not be read reports: nothing
    # but the gem's failure.
    UNREAD = EngineDiagnostics::Report.new(has_errors: false, error: nil, diagnostics: [].freeze)

    # @api private: {XlsxReader.open} makes the book
    def initialize(capi, handle, timezone)
      @capi = capi
      @handle = handle
      @timezone = timezone
      @lock = Mutex.new
      # The rows handles open, each freed by its sheet's read or at the end.
      @rows = []
      # Why the gem stopped short where the engine keeps no failure.
      @failure = nil
      @closed = false
    end

    # The sheets, in the order the workbook lists them, hidden ones
    # included; none when the workbook cannot be read.
    #
    # @return [Array<SheetInfo>]
    def sheets
      status, sheets = call { @capi.book_sheets(@handle) }
      return [] unless status == StatusDiagnostic::OK

      sheets.each_with_index.map do |(name, kind, state), index|
        SheetInfo.new(index:, name:, kind: KINDS.fetch(kind, "unknown"),
                      state: STATES.fetch(state, "unknown"))
      end
    end

    # The sheet named `which` (letter case aside) or at index `which` (from
    # 0, hidden sheets and chartsheets counted); the first when not given.
    # Nothing is read until {Sheet#each_row}. A name or an index no sheet
    # has is the engine's to refuse there; to look for a sheet, look in
    # {#sheets}: a refused sheet fails the {Result}.
    #
    # @param which [String, Integer]
    # @return [Sheet]
    # @raise [ArgumentError] for neither a String nor an Integer from 0
    def sheet(which = 0)
      raise ClosedError, CLOSED if @closed

      Sheet.new(self, which)
    end

    # @api private: the rest is what {Sheet} and {XlsxReader} call

    # @return [Integer, nil] the rows handle, nil when the sheet was refused
    def open_rows(which)
      status, rows = call do
        which.is_a?(String) ? @capi.rows_open_name(@handle, which) : @capi.rows_open(@handle, which)
      end
      return unless status == StatusDiagnostic::OK

      @rows << rows
      rows
    end

    # @return [Row, nil] the next row; nil at the end of the sheet, or when
    #   reading it stopped short, which the {Result} says
    def next_row(rows)
      call do
        status, number, address, count = @capi.rows_next(rows)
        next [status, nil] unless status == StatusDiagnostic::OK && number.positive?

        values = values(number, address, count)
        [status, values && Row.new(number:, values:)]
      end.last
    end

    # Frees a rows handle {#open_rows} made, unless the book's close did.
    def close_rows(rows)
      locked { failed(@capi.rows_close(rows)) if @rows.delete(rows) }
    end

    # Reads the diagnostics, then closes the book, which closes its rows,
    # and frees what is left of each rows handle. Once: a second close does
    # nothing.
    def close
      locked do
        next if @closed

        @closed = true
        @report = report
        failed(@capi.book_close(@handle))
        failed(@capi.rows_close(@rows.shift)) until @rows.empty?
      end
    end

    # @return [Result] `value` on success. Its error is the first failure;
    #   the engine's words for it when the engine kept it too.
    def result(value)
      report = @report || UNREAD
      error = first_error(report.error)
      return Result.failure(report.diagnostics, error:) if error || report.has_errors

      Result.success(value, diagnostics: report.diagnostics)
    end

    private

    # Under the lock, and not cut short by Thread#raise or Timeout: a handle
    # made and not kept, or freed and still listed, would leak or be freed
    # twice.
    def locked(&)
      @lock.synchronize { Thread.handle_interrupt(Object => :never, &) }
    end

    # Calls into the library: its status and what it wrote, the status kept
    # when it is a failure.
    def call
      locked do
        raise ClosedError, CLOSED if @closed

        written = yield
        failed(written.first)
        written
      end
    end

    # Row `number`'s values, copied out; nil when the gem cannot give one,
    # kept as the book's failure.
    def values(number, address, count)
      CellValues.call(address, count, @timezone).freeze
    rescue CellValues::Refused => e
      fail_with(StatusDiagnostic.of(e.code, "row #{number}: #{e.message}"))
      nil
    end

    # Keeps `status` as the book's failure unless it is OK or one is kept.
    # The host's reason is read at once: it is the thread's, and the next
    # failure on the thread replaces it.
    def failed(status)
      return if status == StatusDiagnostic::OK

      message = @capi.error_message if status == StatusDiagnostic::IO
      fail_with(StatusDiagnostic.call(status, message:))
    end

    # The gem's first failure, or the engine's: the engine's words for it
    # when the engine kept the same.
    def first_error(engine)
      return engine if @failure.nil? || @failure.code == engine&.code

      @failure
    end

    # The first failure is the one kept.
    def fail_with(diagnostic)
      @failure = diagnostic if @failure.nil?
    end

    # The book's diagnostics, nil when they cannot be read.
    def report
      status, json = @capi.book_diagnostics(@handle)
      return EngineDiagnostics.call(json) if status == StatusDiagnostic::OK

      failed(status)
      nil
    rescue JSON::ParserError, KeyError, TypeError, NoMethodError => e
      fail_with(StatusDiagnostic.of("diagnostics_unreadable",
                                    "the engine's diagnostics could not be read: #{e.message}"))
      nil
    end
  end
end
