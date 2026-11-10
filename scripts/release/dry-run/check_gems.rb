# frozen_string_literal: true

# Checks each gem under dist/release/packages/ruby the way rubygems.org does:
# the gemspec read back from the built gem passes
# `Gem::Specification#validate(packaging = false, strict = true)` (packaging is
# false because the gem's files are not extracted to disk; strict makes a
# warning fail the check), and carries the version of
# sdk/ruby/lib/xlsx_graft/version.rb and the project's licenses.
#
#   ruby check_gems.rb <the repository> <the directory of the gems>
#
# Every failure is printed, then the exit status is 1.

require "rubygems/package"

root, directory = ARGV
version = File.read(File.join(root, "sdk/ruby/lib/xlsx_graft/version.rb"))[/VERSION = "([^"]+)"/, 1]
licenses = %w[MIT Apache-2.0 BSD-3-Clause]
problems = []

gems = Dir[File.join(directory, "*.gem")].sort
problems << "no gem in #{directory}" if gems.empty?

gems.each do |path|
  name = File.basename(path)
  spec = Gem::Package.new(path).spec
  begin
    spec.validate(false, true)
  rescue Gem::InvalidSpecificationException => e
    problems << "#{name}: #{e.message}"
  end
  problems << "#{name}: version #{spec.version}, not #{version}" unless spec.version.to_s == version
  problems << "#{name}: licenses #{spec.licenses}, not #{licenses}" unless spec.licenses == licenses
  puts "#{name}: #{spec.files.size} files"
end

problems.each { |problem| warn "check_gems: #{problem}" }
puts "check_gems: #{gems.size} gems, #{problems.size} problems"
exit(problems.empty? ? 0 : 1)
