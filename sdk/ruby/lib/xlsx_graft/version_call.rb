# frozen_string_literal: true

require "fiddle"

module XlsxGraft
  # Calls `xlsx_graft_version` and returns the engine's version as a {Result}.
  class VersionCall
    SIGNATURE = [[Fiddle::TYPE_VOIDP, Fiddle::TYPE_VOIDP], Fiddle::TYPE_INT32_T].freeze

    class << self
      # @param library [Library]
      # @return [Result] the version string, such as "0.0.0", on success
      def call(library)
        function = library.function("xlsx_graft_version", *SIGNATURE)
        out_ptr = Fiddle::Pointer.malloc(Fiddle::SIZEOF_VOIDP, Fiddle::RUBY_FREE)
        out_len = Fiddle::Pointer.malloc(Fiddle::SIZEOF_SIZE_T, Fiddle::RUBY_FREE)
        status = function.call(out_ptr, out_len)
        return Result.failure([StatusDiagnostic.call(status)]) unless status == StatusDiagnostic::OK

        Result.success(read_string(out_ptr, out_len))
      end

      private

      # The bytes are not NUL-terminated, so exactly `length` of them are
      # copied out. "J" is an unsigned integer the width of a pointer, which is
      # the width of size_t on every platform the engine builds for.
      def read_string(out_ptr, out_len)
        length = out_len[0, Fiddle::SIZEOF_SIZE_T].unpack1("J")
        out_ptr.ptr.to_str(length).force_encoding(Encoding::UTF_8).freeze
      end
    end
  end
end
