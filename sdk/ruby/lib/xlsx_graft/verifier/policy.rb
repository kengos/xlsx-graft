# frozen_string_literal: true

require "json"

module XlsxGraft
  class Verifier
    # A policy, or the rules of a call, as the engine takes it: JSON whose
    # keys are camelCase.
    class Policy
      class << self
        # @param policy [Hash] keys in snake_case or camelCase, those of the
        #   Hashes inside it and inside its Arrays too; the values are left
        #   as they are
        # @return [String] the policy as JSON
        # @raise [ArgumentError] for a policy that is not a Hash, or holds a
        #   value JSON cannot write
        def call(policy)
          unless policy.is_a?(Hash)
            raise ArgumentError, "policy: is a Hash or nil, not #{policy.class}"
          end

          json(camelized(policy))
        end

        # JSON.generate, with a value it cannot write the caller's
        # ArgumentError.
        #
        # @return [String]
        def json(value)
          JSON.generate(value)
        rescue JSON::GeneratorError, JSON::NestingError => e
          raise ArgumentError, "the policy or the rules cannot be written as JSON: #{e.message}"
        end

        private

        # Recurses as deep as the caller's Hashes and Arrays nest, with no
        # limit of its own.
        def camelized(value)
          case value
          when Hash then value.to_h { |key, item| [camel(key.to_s), camelized(item)] }
          when Array then value.map { |item| camelized(item) }
          else value
          end
        end

        # max_formula_bytes becomes maxFormulaBytes; a camelCase key stays.
        def camel(key)
          key.gsub(/_([a-z\d])/) { Regexp.last_match(1).upcase }
        end
      end
    end
  end
end
