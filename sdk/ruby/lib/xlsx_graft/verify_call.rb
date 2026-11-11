# frozen_string_literal: true

require "fiddle"

module XlsxGraft
  # The C ABI's verify functions (engine/capi/include/xlsx_graft.h, § VERIFY).
  # Each method makes the verify handle, copies its result's JSON out and
  # closes the handle before it returns, so no handle outlives the call
  # (docs/sdk.md § Verifying through the C ABI). Every String goes to the
  # library as a copy of its own ({Capi::Input}).
  #
  # It does not change after it is made, so any thread may use it at once.
  class VerifyCall
    VOIDP = Fiddle::TYPE_VOIDP
    SIZE_T = Fiddle::TYPE_SIZE_T

    SIGNATURES = {
      verify_path: [VOIDP, SIZE_T, VOIDP, SIZE_T, VOIDP],
      verify_bytes: [VOIDP, SIZE_T, VOIDP, SIZE_T, VOIDP],
      policy_check: [VOIDP, SIZE_T, VOIDP],
      verify_result: [VOIDP, VOIDP, VOIDP],
      verify_close: [VOIDP]
    }.freeze

    # @param library [Library]
    # @raise [LibraryNotFound] when the library lacks one of the functions
    def initialize(library)
      @functions = SIGNATURES.to_h do |name, args|
        [name, library.function("xlsx_graft_#{name}", args, Fiddle::TYPE_INT32_T)]
      end.freeze
      freeze
    end

    # @param path [String] UTF-8 text
    # @param options [String] the options JSON, `{"policy": ..., "rules": ...}`
    # @return [Array(Integer, String), Array(Integer, nil)] the status, and
    #   the result's JSON when it is OK
    def path(path, options)
      made(:verify_path, *Capi::Input.call(path, utf8: true), *Capi::Input.call(options))
    end

    # @param data [String] the workbook's bytes
    # @param options [String] as for {#path}
    # @return [Array(Integer, String), Array(Integer, nil)] as {#path}
    def bytes(data, options)
      made(:verify_bytes, *Capi::Input.call(data), *Capi::Input.call(options))
    end

    # @param policy [String] the policy's JSON, not wrapped in `{"policy": ...}`
    # @return [Array(Integer, String), Array(Integer, nil)] as {#path}
    def policy(policy)
      made(:policy_check, *Capi::Input.call(policy))
    end

    private

    # Calls `name`, copies out the result of the handle it wrote and closes
    # the handle. Interrupts are held off from the call to the close, so a
    # handle made is always closed.
    def made(name, *)
      Thread.handle_interrupt(Object => :never) do
        out = Capi.slot
        status = @functions.fetch(name).call(*, out)
        status == StatusDiagnostic::OK ? lent_and_closed(Capi.address(out)) : [status, nil]
      end
    end

    def lent_and_closed(handle)
      lent(handle)
    ensure
      @functions.fetch(:verify_close).call(handle)
    end

    # The status of verify_result, and the handle's JSON copied out (nil
    # when the status is not OK).
    def lent(handle)
      out_ptr = Capi.slot
      out_len = Capi.slot
      status = @functions.fetch(:verify_result).call(handle, out_ptr, out_len)
      ok = status == StatusDiagnostic::OK
      [status, (Capi.text(Capi.address(out_ptr), Capi.address(out_len)) if ok)]
    end
  end
end
