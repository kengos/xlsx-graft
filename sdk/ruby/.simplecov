# frozen_string_literal: true

# The coverage gate (docs/guidelines.md § Test coverage). SimpleCov loads this file
# itself when spec/spec_helper.rb calls `SimpleCov.start`.
SimpleCov.start do
  enable_coverage :line
  add_filter "/spec/"
  minimum_coverage line: 100
end
