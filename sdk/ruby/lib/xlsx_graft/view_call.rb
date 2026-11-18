# frozen_string_literal: true

require "fiddle"

module XlsxGraft
  # The C ABI's view functions (engine/capi/include/xlsx_graft.h, § VIEWS).
  # Each call makes the view handle, copies its result's JSON out and closes
  # the handle before it returns, so no handle outlives the call
  # (docs/sdk.md § Viewing through the C ABI). Every String goes to the
  # library as a copy of its own ({Capi::Input}).
  #
  # A ViewCall does not change after it is made, so any thread may use it at
  # once.
  class ViewCall
    VOIDP = Fiddle::TYPE_VOIDP
    SIZE_T = Fiddle::TYPE_SIZE_T

    SIGNATURES = {
      tree_path: [VOIDP, SIZE_T, VOIDP, SIZE_T, VOIDP],
      tree_bytes: [VOIDP, SIZE_T, VOIDP, SIZE_T, VOIDP],
      describe_sheet_path: [VOIDP, SIZE_T, VOIDP, SIZE_T, VOIDP],
      describe_sheet_bytes: [VOIDP, SIZE_T, VOIDP, SIZE_T, VOIDP],
      view_result: [VOIDP, VOIDP, VOIDP],
      view_close: [VOIDP],
      error_message: [VOIDP, VOIDP]
    }.freeze

    # @param library [Library]
    # @raise [LibraryNotFound] when the library lacks one of the functions
    def initialize(library)
      @functions = SIGNATURES.to_h do |name, args|
        [name, library.function("xlsx_graft_#{name}", args, Fiddle::TYPE_INT32_T)]
      end.freeze
      freeze
    end

    # @param name [Symbol] `:tree_path`, `:tree_bytes`, `:describe_sheet_path`
    #   or `:describe_sheet_bytes`
    # @param input [String] for a `_path` name the path as UTF-8 text, for a
    #   `_bytes` name the workbook's bytes
    # @param options [String] the options' JSON; "" is every default
    # @return [Array(Integer, String), Array(Integer, nil)] the status, and
    #   the result's JSON when it is OK, or the message of
    #   XLSX_GRAFT_ERR_REQUEST_INVALID (read on this thread right after the
    #   call, as the header says; nil when it cannot be read); nil for any
    #   other status
    def call(name, input, options)
      made(name, *Capi::Input.call(input, utf8: name.end_with?("_path")),
           *Capi::Input.call(options))
    end

    private

    # Calls `name`, copies out the result of the handle it wrote and closes
    # the handle. For XLSX_GRAFT_ERR_REQUEST_INVALID it reads
    # xlsx_graft_error_message instead, within the same handle_interrupt
    # block. Interrupts are held off from the call to the close, so a handle
    # made is always closed.
    def made(name, *)
      Thread.handle_interrupt(Object => :never) do
        out = Capi.slot
        status = @functions.fetch(name).call(*, out)
        case status
        when StatusDiagnostic::OK then lent_and_closed(Capi.address(out))
        when StatusDiagnostic::REQUEST_INVALID then [status, message]
        else [status, nil]
        end
      end
    end

    def lent_and_closed(handle)
      lent(handle)
    ensure
      @functions.fetch(:view_close).call(handle)
    end

    # The status of view_result, and the handle's JSON copied out (nil when
    # the status is not OK).
    def lent(handle)
      out_ptr = Capi.slot
      out_len = Capi.slot
      status = @functions.fetch(:view_result).call(handle, out_ptr, out_len)
      ok = status == StatusDiagnostic::OK
      [status, (Capi.text(Capi.address(out_ptr), Capi.address(out_len)) if ok)]
    end

    # Why the call that returned XLSX_GRAFT_ERR_REQUEST_INVALID failed; nil
    # when xlsx_graft_error_message does not return OK.
    def message
      out_ptr = Capi.slot
      out_len = Capi.slot
      status = @functions.fetch(:error_message).call(out_ptr, out_len)
      return unless status == StatusDiagnostic::OK

      Capi.text(Capi.address(out_ptr), Capi.address(out_len))
    end
  end
end
