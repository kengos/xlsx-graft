# frozen_string_literal: true

require "fiddle"

module XlsxGraft
  class Capi
    # What {Capi} hands the library for a String: a copy of its own, and its
    # length. Fiddle lets go of the GVL during a call, so another thread
    # could change the caller's String while the engine reads it; no thread
    # can change the copy. No bytes go as NULL, as the header allows. The
    # copy is freed when Ruby collects it, after the call.
    class Input
      class << self
        # @param string [String]
        # @param utf8 [Boolean] whether to send the text as UTF-8 (a path, a
        #   sheet name) rather than its bytes as they are (a workbook). Text
        #   with no UTF-8 form goes as it is, and the library refuses it
        #   with XLSX_GRAFT_ERR_INVALID_ARG.
        # @return [Array(Fiddle::Pointer, Integer), Array(nil, 0)]
        def call(string, utf8: false)
          bytes = utf8 ? encoded(string) : string
          return [nil, 0] if bytes.empty?

          copy = Fiddle::Pointer.malloc(bytes.bytesize, Fiddle::RUBY_FREE)
          copy[0, bytes.bytesize] = bytes
          [copy, bytes.bytesize]
        end

        private

        def encoded(text)
          text.encode(Encoding::UTF_8)
        rescue EncodingError
          text
        end
      end
    end
  end
end
