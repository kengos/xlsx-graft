# frozen_string_literal: true

# Run by `make sdk:ruby:verify` after it installs the built gem into a
# directory of its own: loads the gem from there, with no bundler and no
# checkout on the load path, and checks it reaches the engine.
#
#   ruby script/check_installed_gem.rb <gem-dir> <engine/Cargo.toml>

gem_dir, cargo_toml = ARGV
engine = File.read(cargo_toml)[/^version = "([^"]+)"/, 1]

require "xlsx_graft"

spec = Gem.loaded_specs.fetch("xlsx_graft")
where = spec.full_gem_path
abort "loaded xlsx_graft from #{where}, not from #{gem_dir}" unless where.start_with?("#{gem_dir}/")

# fiddle loads here whether or not the gem declares it (a default gem on 3.4,
# a bundled one on 4.0), so the declaration is checked by name.
dependencies = spec.runtime_dependencies.map(&:name)
unless dependencies == ["fiddle"]
  abort "xlsx_graft depends at run time on #{dependencies}, not [\"fiddle\"]"
end

result = XlsxGraft.version
abort "XlsxGraft.version failed: #{result.diagnostics.inspect}" unless result.success?
abort "XlsxGraft.version is #{result.value}, the engine is #{engine}" unless result.value == engine

puts "installed xlsx_graft #{XlsxGraft::VERSION} from #{where} loads engine #{result.value}"
