# frozen_string_literal: true

require "json"

module XlsxGraft
  # Writes the options of a `describe_sheet` call as the JSON the engine
  # takes (engine/capi/include/xlsx_graft.h, § VIEWS), and refuses the values
  # the engine could not read as JSON, so that a fault of the caller's
  # argument is an ArgumentError before the C ABI is called.
  class ViewOptions
    # The most JSON the engine takes as options.
    MAX_BYTES = 2 << 20

    class << self
      # @param sheet [Integer, String, Symbol, nil] an Integer is the sheet's
      #   index (`sheetIndex`); anything else that is not nil is its name
      #   (`sheet`)
      # @param range [String, nil]
      # @param max_rows [Integer, nil]
      # @param max_columns [Integer, nil]
      # @return [String] the options' JSON; the keys of the values that are
      #   nil are not written
      # @raise [ArgumentError] when an Integer is beyond what a double holds,
      #   a value is not JSON that can be written, or the JSON is more than
      #   2 MiB
      def call(sheet:, range:, max_rows:, max_columns:)
        entries = { (sheet.is_a?(Integer) ? "sheetIndex" : "sheet") => ["sheet", sheet],
                    "range" => ["range", range], "maxRows" => ["max_rows", max_rows],
                    "maxColumns" => ["max_columns", max_columns] }
                  .reject { |_, (_, value)| value.nil? }
        json = json(entries.transform_values(&:last))
        entries.each_value { |keyword, value| within_double!(keyword, value) }
        sized(json)
      end

      private

      # The engine reads a JSON number as a double, and takes one beyond a
      # double's range as no JSON at all.
      # Raises ArgumentError for an Integer beyond a double's range, also one
      # inside an Array or a Hash.
      def within_double!(keyword, value)
        case value
        when Array then value.each { |item| within_double!(keyword, item) }
        when Hash then value.each_value { |item| within_double!(keyword, item) }
        when Integer then beyond_double!(keyword, value)
        end
      end

      def beyond_double!(keyword, value)
        return unless value.abs > Float::MAX

        digits = value.abs.to_s.length
        raise ArgumentError, "#{keyword}: an Integer of #{digits} digits is beyond " \
                             "what the engine reads"
      end

      def json(options)
        JSON.generate(options)
      rescue JSON::GeneratorError, JSON::NestingError, EncodingError => e
        raise ArgumentError, "the options cannot be written as JSON: #{e.message}"
      end

      def sized(json)
        return json if json.bytesize <= MAX_BYTES

        raise ArgumentError, "the options are #{json.bytesize} bytes of JSON; " \
                             "the engine takes #{MAX_BYTES}"
      end
    end
  end
end
