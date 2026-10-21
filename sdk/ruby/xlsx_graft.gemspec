# frozen_string_literal: true

# Read rather than required: requiring it would define XlsxGraft::VERSION in
# whatever process evaluates this gemspec, and the specs then load the same
# constant again from lib/ and warn about the redefinition.
XLSX_GRAFT_VERSION = File.read(File.expand_path("lib/xlsx_graft/version.rb", __dir__))[
  /VERSION = "([^"]+)"/, 1
]
XLSX_GRAFT_REPO = "https://github.com/kengos/xlsx-graft"

Gem::Specification.new do |spec|
  spec.name = "xlsx_graft"
  spec.version = XLSX_GRAFT_VERSION
  spec.authors = ["kengos"]

  spec.summary = "Read, verify, create and edit xlsx workbooks"
  spec.description = <<~TEXT
    Ruby bindings for xlsx-graft, an engine that reads, verifies, creates and
    edits xlsx workbooks. The engine is a shared C library loaded through
    fiddle, so this gem behaves the same as the CLI and every other
    xlsx-graft SDK.
  TEXT
  spec.homepage = XLSX_GRAFT_REPO
  # The workspace's `MIT OR Apache-2.0 OR BSD-3-Clause`, as RubyGems spells
  # it: a list of SPDX identifiers, read as "any of these, at your option".
  spec.licenses = ["MIT", "Apache-2.0", "BSD-3-Clause"]

  # docs/sdk.md § Supported language versions.
  spec.required_ruby_version = ">= 3.4.0"

  spec.metadata = {
    "homepage_uri" => spec.homepage,
    "source_code_uri" => "#{XLSX_GRAFT_REPO}/tree/main/sdk/ruby",
    "rubygems_mfa_required" => "true"
  }

  # The second glob is the engine a platform gem ships in
  # lib/xlsx_graft/native/. In a checkout the directory does not exist and the
  # glob adds nothing.
  spec.files = Dir["lib/**/*.rb", "lib/xlsx_graft/native/*", "README.md"]
  spec.require_paths = ["lib"]

  # Declared rather than assumed: fiddle is no longer a default gem from Ruby
  # 4.0, so a gem that relied on it being there would fail to load.
  spec.add_dependency "fiddle", "~> 1.1"
end
