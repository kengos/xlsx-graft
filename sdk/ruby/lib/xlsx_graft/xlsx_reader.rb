# frozen_string_literal: true

module XlsxGraft
  # Opens a workbook to read its sheets' rows, through the C ABI.
  #
  # @example
  #   result = XlsxGraft::XlsxReader.open("book.xlsx", timezone: "+09:00") do |book|
  #     book.sheet("Data").each_row { |row| puts "#{row.number}: #{row.values.inspect}" }
  #   end
  #   result.diagnostics.each { |d| warn "#{d.severity} #{d.code}: #{d.message}" }
  #   warn result.error.message if result.error
  class XlsxReader
    class << self
      # Opens the workbook at `path`, yields it as a {Book}, and closes it
      # when the block ends, however it ends. An exception the block raises
      # is the caller's: it propagates, the book closed first.
      #
      # @param path [String, #to_path]
      # @param timezone [String, #local_to_utc] the zone a date-time cell is
      #   read in: anything Time.new takes as `in:` ("UTC", "+09:00", a
      #   timezone object). The workbook's are wall-clock times with no zone.
      #   A timezone object is called under the book's lock, where neither
      #   Timeout nor Thread#raise reaches it: it must not use the book, nor
      #   wait.
      # @param library [String, nil] the engine library's path, used when
      #   XLSX_GRAFT_LIBRARY is not set
      # @yieldparam book [Book]
      # @return [Result] the block's value on success (let it through: a
      #   `break` out of the block returns its own value, not the Result). It
      #   fails with `error` set when the file cannot be read, the workbook or
      #   a sheet asked for is refused, or the engine fails; and without,
      #   when a problem stopped a sheet's rows, its diagnostic an error.
      # @raise [ArgumentError] without a block, or for a zone Time does not
      #   take
      # @raise [LibraryNotFound] when no engine library is found
      def open(path, timezone: "UTC", library: nil, &)
        read(:book_open_path, File.path(path), timezone, library, &)
      end

      # As {open}, the workbook being the bytes of `data`, which the engine
      # copies.
      #
      # @param data [String]
      def open_bytes(data, timezone: "UTC", library: nil, &)
        read(:book_open_bytes, data, timezone, library, &)
      end

      private

      def read(how, input, timezone, library, &)
        raise ArgumentError, "a block is required: the book is closed when it ends" unless
          block_given?

        zone(timezone)
        opened(Capi.new(Library.open(path: library)), how, input, timezone, &)
      end

      # Only the open itself, up to the book that keeps its handle, is kept
      # from interrupts: a handle made and not kept would leave its file open.
      # One held back fires as that ends, inside the begin whose ensure closes
      # the book. The block takes interrupts as the caller's thread does.
      # Kept in one method: the book has to be in a variable the ensure sees.
      def opened(capi, how, input, timezone) # rubocop:disable Metrics/MethodLength
        book = nil
        begin
          refusal = Thread.handle_interrupt(Object => :never) do
            status, handle = capi.public_send(how, input)
            book = Book.new(capi, handle, timezone) if status == StatusDiagnostic::OK
            refused(capi, status) unless book
          end
          return refusal unless book

          value = yield book
        ensure
          Thread.handle_interrupt(Object => :never) { book.close } if book
        end
        book.result(value)
      end

      def zone(timezone)
        Time.new(2000, in: timezone)
      rescue TypeError, NoMethodError => e
        raise ArgumentError, "timezone: #{timezone.inspect} is no zone Time takes (#{e.message})"
      end

      def refused(capi, status)
        message = capi.error_message if status == StatusDiagnostic::IO
        Result.failure([], error: StatusDiagnostic.call(status, message:))
      end
    end
  end
end
