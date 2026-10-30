# frozen_string_literal: true

require "fiddle"

module XlsxGraft
  # The C ABI's book and rows functions (engine/capi/include/xlsx_graft.h),
  # declared with their types. Each method makes one call and returns its
  # status first, then what it wrote; what it wrote is only meaningful when
  # the status is XLSX_GRAFT_OK.
  #
  # A handle is the address the library wrote, an Integer; 0 is NULL. What
  # the library lends (sheet names, the diagnostics' JSON) is copied out
  # before the method returns. A row's values are not: {CellValues} reads
  # them, before the next call on that rows handle, or its close.
  #
  # Every String goes to the library as a copy of its own ({Input}).
  #
  # Thread safety is the caller's: nothing here locks.
  class Capi
    VOIDP = Fiddle::TYPE_VOIDP
    SIZE_T = Fiddle::TYPE_SIZE_T

    SIGNATURES = {
      book_open_path: [VOIDP, SIZE_T, VOIDP],
      book_open_bytes: [VOIDP, SIZE_T, VOIDP],
      book_sheets: [VOIDP, VOIDP, VOIDP],
      book_diagnostics: [VOIDP, VOIDP, VOIDP],
      book_close: [VOIDP],
      rows_open: [VOIDP, SIZE_T, VOIDP],
      rows_open_name: [VOIDP, VOIDP, SIZE_T, VOIDP],
      rows_next: [VOIDP, VOIDP, VOIDP, VOIDP],
      rows_close: [VOIDP],
      error_message: [VOIDP, VOIDP]
    }.freeze

    # XlsxGraftSheet: name, name_len, kind, state (24 bytes on a 64-bit
    # target, the only one the engine is built for).
    SHEET_LAYOUT = "JJLL"
    SHEET_SIZE = 24

    class << self
      # The `length` UTF-8 bytes at `address`, copied out, never read as a C
      # string. An empty text may come as NULL.
      #
      # @return [String]
      def text(address, length)
        return +"" if length.zero?

        Fiddle::Pointer.new(address).to_str(length).force_encoding(Encoding::UTF_8)
      end
    end

    # @param library [Library]
    # @raise [LibraryNotFound] when the library lacks one of the functions
    def initialize(library)
      raise LibraryNotFound, "this gem reads the engine's 64-bit layout only" unless
        Fiddle::SIZEOF_VOIDP == 8

      @functions = SIGNATURES.to_h do |name, args|
        [name, library.function("xlsx_graft_#{name}", args, Fiddle::TYPE_INT32_T)]
      end
    end

    # @param path [String] a String, not a Pathname
    # @return [Array(Integer, Integer)] the status and the book handle
    def book_open_path(path)
      open_handle(:book_open_path, *Input.call(path, utf8: true))
    end

    # @param data [String] the workbook's bytes
    # @return [Array(Integer, Integer)] the status and the book handle
    def book_open_bytes(data)
      open_handle(:book_open_bytes, *Input.call(data))
    end

    # @return [Array(Integer, Array<Array(String, Integer, Integer)>)] the
    #   status and each sheet's name, kind and state
    def book_sheets(book)
      status, address, count = lent(:book_sheets, book)
      return [status, []] if count.zero?

      bytes = Fiddle::Pointer.new(address).to_str(SHEET_SIZE * count)
      sheets = Array.new(count) do |index|
        name, name_len, kind, state = bytes.unpack(SHEET_LAYOUT, offset: index * SHEET_SIZE)
        [Capi.text(name, name_len), kind, state]
      end
      [status, sheets]
    end

    # @return [Array(Integer, String)] the status and the diagnostics' JSON
    def book_diagnostics(book)
      status, address, length = lent(:book_diagnostics, book)
      [status, Capi.text(address, length)]
    end

    # @return [Integer] the status
    def book_close(book)
      @functions.fetch(:book_close).call(book)
    end

    # @param index [Integer] from 0 to what a size_t holds
    # @return [Array(Integer, Integer)] the status and the rows handle
    def rows_open(book, index)
      open_handle(:rows_open, book, index)
    end

    # @param name [String]
    # @return [Array(Integer, Integer)] the status and the rows handle
    def rows_open_name(book, name)
      open_handle(:rows_open_name, book, *Input.call(name, utf8: true))
    end

    # @return [Array(Integer, Integer, Integer, Integer)] the status, the
    #   row's number (0 at the end), and the address and count of its values
    def rows_next(rows)
      number = Fiddle::Pointer.malloc(4, Fiddle::RUBY_FREE)
      values = slot
      count = slot
      status = @functions.fetch(:rows_next).call(rows, number, values, count)
      return [status, 0, 0, 0] unless status == StatusDiagnostic::OK

      [status, number[0, 4].unpack1("L"), address(values), address(count)]
    end

    # @return [Integer] the status
    def rows_close(rows)
      @functions.fetch(:rows_close).call(rows)
    end

    # Why the last call on this thread that returned XLSX_GRAFT_ERR_IO
    # failed: the operating system's thread, so call it right after the
    # call that failed (under Ruby's M:N threads a Ruby thread may move
    # between them across calls).
    #
    # @return [String]
    def error_message
      _status, address, length = lent(:error_message)
      Capi.text(address, length)
    end

    private

    # A pointer-sized out-parameter. "J" is an unsigned integer the width
    # of a pointer, which is the width of size_t on every platform the
    # engine builds for.
    def slot
      Fiddle::Pointer.malloc(Fiddle::SIZEOF_VOIDP, Fiddle::RUBY_FREE)
    end

    def address(slot)
      slot[0, Fiddle::SIZEOF_VOIDP].unpack1("J")
    end

    def open_handle(name, *)
      out = slot
      status = @functions.fetch(name).call(*, out)
      [status, status == StatusDiagnostic::OK ? address(out) : 0]
    end

    # Calls a function whose last two parameters are a pointer and a length
    # it writes.
    def lent(name, *)
      out_ptr = slot
      out_len = slot
      status = @functions.fetch(name).call(*, out_ptr, out_len)
      return [status, 0, 0] unless status == StatusDiagnostic::OK

      [status, address(out_ptr), address(out_len)]
    end
  end
end
