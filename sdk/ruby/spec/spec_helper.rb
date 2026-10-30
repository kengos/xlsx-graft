# frozen_string_literal: true

require "simplecov"

# The threshold lives in .simplecov, which SimpleCov loads itself.
SimpleCov.start

require "xlsx_graft"
require "fileutils"
require "tmpdir"
require_relative "support/workbook"

# The engine library `make engine:capi-lib` built; the gate points
# XLSX_GRAFT_LIBRARY at it.
ENGINE_LIBRARY = ENV.fetch(XlsxGraft::Library::ENV_VAR)

RSpec.configure do |config|
  config.expect_with(:rspec) { |expectations| expectations.syntax = :expect }
  config.disable_monkey_patching!
  config.order = :random
  Kernel.srand config.seed
end
