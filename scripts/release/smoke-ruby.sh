#!/bin/sh
# Installs the platform gem of the Docker host's platform into a directory of
# its own and loads it with no XLSX_GRAFT_LIBRARY, so the only engine it can
# reach is the library the gem carries. Runs inside the Ruby gate image.
#
#   scripts/release/smoke-ruby.sh <slug> <engine version>
set -eu

slug=$1
want=$2
gem_file=$(ls "/repo/dist/release/packages/ruby/xlsx_graft-"*"-$(/repo/scripts/release/platform.sh "$slug" gem).gem")

unset BUNDLE_GEMFILE XLSX_GRAFT_LIBRARY
gem_path=$(gem env gempath)
export GEM_PATH="/tmp/gems:$gem_path"
export GEM_HOME=/tmp/gems
# --local: fiddle comes from the image's gems, nothing is fetched.
gem install --local --no-document "$gem_file"
cd /tmp
ruby -e '
  require "xlsx_graft"
  result = XlsxGraft.version
  abort "XlsxGraft.version failed: #{result.diagnostics.inspect}" unless result.success?
  abort "engine is #{result.value}, not #{ARGV[0]}" unless result.value == ARGV[0]
  puts "platform gem loads engine #{result.value}"
' "$want"
