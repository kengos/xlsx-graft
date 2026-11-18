# frozen_string_literal: true

module XlsxGraft
  # A path the caller gave, as the C ABI can take it: UTF-8 text goes as the
  # path itself, and a path that is not UTF-8 goes as the bytes of the file
  # the gem reads, since the engine takes no path that is not UTF-8.
  class PathInput
    # The encodings whose paths are taken as UTF-8 when their bytes are valid UTF-8.
    UTF8_COMPATIBLE = [Encoding::UTF_8, Encoding::US_ASCII, Encoding::BINARY].freeze

    class << self
      # @param path [String, #to_path]
      # @yieldparam how [Symbol] `:path` or `:bytes`
      # @yieldparam input [String] the path as UTF-8 text, or the file's bytes
      # @return [Object] what the block returned, or an `io` failure
      #   {Result} (the block is not called) when the file cannot be read
      def call(path)
        path = File.path(path)
        utf8 = utf8(path)
        return yield(:path, utf8) if utf8

        read(path) { |data| yield(:bytes, data) }
      end

      private

      # `path` as UTF-8 text when its bytes are UTF-8, else nil. A path marked
      # with another encoding is not converted: Ruby opens a file by a path's
      # bytes, so the engine has to be given those bytes or none.
      def utf8(path)
        return unless UTF8_COMPATIBLE.include?(path.encoding)

        text = path.dup.force_encoding(Encoding::UTF_8)
        text if text.valid_encoding?
      end

      # `string`'s bytes as UTF-8 text, a byte that is no UTF-8 replaced.
      def text(string)
        string.b.force_encoding(Encoding::UTF_8).scrub
      end

      # The engine takes no path that is not UTF-8, so the gem reads the file
      # and yields its bytes; a file that cannot be read is an `io` failure.
      def read(path)
        data = File.binread(path)
      rescue SystemCallError => e
        message = "the file could not be read: #{text(e.message)}"
        Result.failure([], error: StatusDiagnostic.of("io", message))
      else
        yield data
      end
    end
  end
end
