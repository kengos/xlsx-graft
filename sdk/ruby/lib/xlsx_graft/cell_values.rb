# frozen_string_literal: true

require "date"
require "fiddle"

module XlsxGraft
  # Turns the XlsxGraftValue array `xlsx_graft_rows_next` lends into Ruby
  # values, copying everything out: the array and its text live only until
  # the next call on the rows handle.
  #
  # | kind     | Ruby                                                  |
  # | -------- | ----------------------------------------------------- |
  # | empty    | nil                                                   |
  # | string   | String                                                |
  # | number   | Float, 3.0 included                                   |
  # | boolean  | true or false                                         |
  # | date     | Date                                                  |
  # | time     | {TimeOfDay}                                           |
  # | datetime | Time, the wall-clock time in the caller's zone        |
  # | elapsed  | Float of seconds, not wrapped at a day (milliseconds  |
  # |          | as a binary fraction: 0.001 is not exact)             |
  # | error    | {CellError}                                           |
  #
  # A date, time or date-time is the workbook's wall-clock time with no time
  # zone. The date-time is read as a time in `timezone:` (UTC unless the
  # caller says otherwise): the zone is the caller's to know, not the
  # workbook's.
  class CellValues
    # kind u32, year u16, month, day, hour, minute, second and boolean u8,
    # millisecond and reserved u16, number f64, elapsed u64, text pointer
    # and text_len: 48 bytes on a 64-bit target.
    LAYOUT = "LSC6S2dQJJ"
    SIZE = 48

    # A value that cannot be given: of a kind this gem does not know (an
    # engine newer than the gem, `unknown_value`), or a date-time a
    # timezone object's rules cannot place (`datetime_unrepresentable`): a
    # time its daylight saving skips, or one it has twice (TZInfo raises
    # for it unless told which). Neither is guessed at.
    class Refused < StandardError
      attr_reader :code

      def initialize(code, message)
        super(message)
        @code = code
      end
    end

    class << self
      # @param address [Integer] where the first value is
      # @param count [Integer] how many there are
      # @param timezone [String, #local_to_utc] what Time.new takes as `in:`
      # @return [Array<Object>]
      # @raise [Refused] when a value cannot be given
      def call(address, count, timezone)
        return [] if count.zero?

        bytes = Fiddle::Pointer.new(address).to_str(SIZE * count)
        Array.new(count) { |index| value(bytes.unpack(LAYOUT, offset: index * SIZE), timezone) }
      end

      private

      # One branch a kind, which the metrics count against it.
      def value(fields, timezone) # rubocop:disable Metrics
        kind, year, month, day, hour, minute, second, boolean, millisecond, _reserved,
          number, elapsed, text, text_len = fields
        case kind
        when 0 then nil
        when 1 then Capi.text(text, text_len).freeze
        when 2 then number
        when 3 then boolean == 1
        when 4 then Date.new(year, month, day)
        when 5 then TimeOfDay.new(hour:, minute:, second:, millisecond:)
        when 6 then moment(year, month, day, hour, minute, second + Rational(millisecond, 1000),
                           timezone)
        when 7 then elapsed / 1000.0
        when 8 then CellError.new(text: Capi.text(text, text_len).freeze)
        else raise Refused.new("unknown_value", "the engine returned a value of kind #{kind}, " \
                                                "which this gem does not know")
        end
      end

      # A timezone object's rules may have no such time, or two of it. A zone
      # Time does not take at all is the caller's mistake, checked before
      # any cell is read, and a NoMethodError or TypeError a bug: neither
      # is a cell's.
      def moment(*, timezone)
        Time.new(*, in: timezone)
      rescue NoMethodError, TypeError
        raise
      rescue StandardError => e
        raise Refused.new("datetime_unrepresentable",
                          "#{Time.utc(*).strftime("%F %T.%L")} cannot be placed in " \
                          "#{timezone.inspect}: #{e.message}")
      end
    end
  end
end
