# frozen_string_literal: true

module XlsxGraft
  # The root of what this gem raises. A call into the engine never raises; it
  # returns a {Result}. What raises is a setup that cannot reach the engine,
  # or a book used after its block. A call made wrongly raises Ruby's own
  # ArgumentError.
  class Error < StandardError; end

  # No engine library could be found or loaded. A configuration mistake, not a
  # failed call, so it is raised rather than returned as a {Result}.
  class LibraryNotFound < Error; end

  # A {Book} or {Sheet} used after the block of {XlsxReader.open} ended,
  # which closed it.
  class ClosedError < Error; end

  # The policy given to {Verifier.new} is one the engine does not take. A
  # setup mistake, as a missing library is, so it is raised.
  class PolicyInvalid < Error
    # @return [Array<Diagnostic>] the policy_invalid diagnostics, each naming
    #   the key (`args["key"]`, such as `forbid.parts[1].kind`) and why; or
    #   the failure of the check itself
    attr_reader :diagnostics

    # @param diagnostics [Array<Diagnostic>]
    def initialize(diagnostics)
      @diagnostics = diagnostics.freeze
      super("the policy is not valid: #{diagnostics.map(&:message).join("; ")}")
    end
  end
end
