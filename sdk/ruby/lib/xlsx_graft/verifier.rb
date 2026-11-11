# frozen_string_literal: true

require_relative "verifier/policy"

module XlsxGraft
  # Verifies workbooks through the C ABI, under the host's policy (given to
  # {#initialize}, and checked there) and the rules of the caller (given to
  # each call).
  #
  # A verifier does not change once it is made, so one may be shared by any
  # number of threads, and their calls may run at once.
  #
  # @example
  #   verifier = XlsxGraft::Verifier.new(policy: { max_formula_bytes: 32_768 })
  #   result = verifier.verify("book.xlsx", targets: ["excel >= 2016"], allow_parts: ["macro"])
  #   result.diagnostics.each { |d| warn "#{d.severity} #{d.code}: #{d.message}" }
  #   warn result.error.message if result.error
  class Verifier
    # The most JSON the engine takes as the policy and the rules together.
    MAX_OPTIONS_BYTES = 2 << 20

    # @param policy [Hash, nil] the host's policy: the engine's limits,
    #   `fail_on`, `deny_scheduled` and `forbid: { functions: [...], parts: [...] }`.
    #   Keys are snake_case or camelCase; the values are not changed. nil is
    #   the engine's defaults.
    # @param library [String, nil] the engine library's path, used when
    #   XLSX_GRAFT_LIBRARY is not set
    # @raise [PolicyInvalid] when the engine does not take the policy
    # @raise [ArgumentError] when the policy is not a Hash or holds a value
    #   JSON cannot write
    # @raise [LibraryNotFound] when no engine library is found
    def initialize(policy: nil, library: nil)
      @call = VerifyCall.new(Library.open(path: library))
      @policy = policy.nil? ? "null" : checked(Policy.call(policy))
      freeze
    end

    # Verifies the workbook at `path`.
    #
    # @param path [String, #to_path]
    # @param targets [Array<String>, nil] the versions the workbook must run
    #   in (`"excel >= 2016"`); a problem with one is an error
    # @param best_effort [Array<String>, nil] the versions it should run in;
    #   a problem with one is a warning
    # @param allow_functions [Array<String>, nil] functions refused by default
    #   to let through (without an `_xlfn.` prefix)
    # @param allow_parts [Array<String>, nil] kinds of part refused by default
    #   to let through (`macro`, `external_link`, `dde`, `ole`, `activex`,
    #   `data_connection`, `web_publish`)
    # @param fail_on [String, Symbol, nil] `"warn"` to fail on a warning too;
    #   the stricter of this and the policy's `fail_on` counts
    # @return [Result] successful when the engine's `ok` is true; it has no
    #   value. A file that cannot be read is an `io` `error`; a fault of the
    #   rules is a `rules_invalid` or `target_invalid` diagnostic, and the
    #   workbook is not opened. (For a path that is not UTF-8 this gem reads
    #   the file, so an unreadable file is an `io` `error` and the rules are
    #   not checked.) With `fail_on: "warn"` a result fails on a
    #   warning with no error diagnostic.
    # @raise [ArgumentError] when the policy and the rules make more than
    #   2 MiB of JSON, or a rule holds a value JSON cannot write
    def verify(path, targets: nil, best_effort: nil, allow_functions: nil, allow_parts: nil,
               fail_on: nil)
      options = options(targets:, best_effort:, allow_functions:, allow_parts:, fail_on:)
      path = File.path(path)
      utf8 = utf8(path)
      return run(:path, utf8, options) if utf8

      read(path) { |data| run(:bytes, data, options) }
    end

    # Verifies the workbook in the bytes of `data`, which this gem copies for
    # the call.
    #
    # @param data [String]
    # @return [Result] (see #verify)
    def verify_bytes(data, targets: nil, best_effort: nil, allow_functions: nil,
                     allow_parts: nil, fail_on: nil)
      run(:bytes, data, options(targets:, best_effort:, allow_functions:, allow_parts:, fail_on:))
    end

    private

    # The policy JSON, when the engine takes it.
    def checked(json)
      result = VerifyReport.call(*@call.policy(json))
      raise PolicyInvalid, [*result.error, *result.diagnostics] if result.failure?

      json.freeze
    end

    # The options JSON: the policy and the rules the caller gave, by the
    # engine's names.
    def options(targets:, best_effort:, allow_functions:, allow_parts:, fail_on:)
      rules = { "targets" => targets, "bestEffort" => best_effort,
                "functions" => allow_functions && { "allow" => allow_functions },
                "parts" => allow_parts && { "allow" => allow_parts },
                "failOn" => fail_on }.compact
      sized(%({"policy":#{@policy},"rules":#{Policy.json(rules)}}))
    end

    def sized(options)
      return options if options.bytesize <= MAX_OPTIONS_BYTES

      raise ArgumentError, "the policy and the rules are #{options.bytesize} bytes of JSON; " \
                           "the engine takes #{MAX_OPTIONS_BYTES}"
    end

    def run(how, input, options)
      VerifyReport.call(*@call.public_send(how, input, options))
    end

    # `path` as UTF-8 text when its bytes are UTF-8, else nil. A path marked
    # with another encoding is not converted: Ruby opens a file by a path's
    # bytes, so the engine has to be given those bytes or none.
    def utf8(path)
      return unless [Encoding::UTF_8, Encoding::US_ASCII, Encoding::BINARY].include?(path.encoding)

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
