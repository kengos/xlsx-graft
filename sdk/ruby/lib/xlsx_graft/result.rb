# frozen_string_literal: true

module XlsxGraft
  # What every call into the engine returns: whether it worked, the value when
  # it did, why it did not when it stopped, and the diagnostics either way.
  #
  # `error` is a {Diagnostic} saying why the call stopped short: the
  # workbook or a sheet asked for was refused, or the host or the engine
  # failed. A call can fail with no `error` too: a read that a problem in
  # its sheet stopped, whose diagnostics hold an error, as
  # `xlsx-graft read`'s `ok` is false then.
  class Result
    attr_reader :value, :error, :diagnostics

    class << self
      def success(value, diagnostics: [])
        new(success: true, value: value, diagnostics: diagnostics)
      end

      def failure(diagnostics, error: nil)
        new(success: false, value: nil, diagnostics: diagnostics, error: error)
      end
    end

    def initialize(success:, value:, diagnostics:, error: nil)
      @success = success
      @value = value
      @error = error
      @diagnostics = diagnostics.freeze
      freeze
    end

    def success?
      @success
    end

    def failure?
      !@success
    end
  end
end
