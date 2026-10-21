# frozen_string_literal: true

module XlsxGraft
  # What every call into the engine returns: whether it worked, the value when
  # it did, and the diagnostics either way.
  class Result
    attr_reader :value, :diagnostics

    class << self
      def success(value, diagnostics: [])
        new(success: true, value: value, diagnostics: diagnostics)
      end

      def failure(diagnostics)
        new(success: false, value: nil, diagnostics: diagnostics)
      end
    end

    def initialize(success:, value:, diagnostics:)
      @success = success
      @value = value
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
