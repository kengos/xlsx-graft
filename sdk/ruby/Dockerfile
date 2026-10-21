# The Ruby SDK's gate environment: `make sdk:*:ruby` builds and runs this
# image, so nobody needs Ruby installed. The build context is sdk/ruby.
#
# The image holds the interpreter and the bundle, and nothing else. The engine
# library is NOT copied in: the gate mounts the repository at /repo and points
# XLSX_GRAFT_LIBRARY at the library `make engine:capi-lib` built there, so
# changing the engine never rebuilds this image. That library is built on
# bookworm, so this is bookworm too: glibc loads a library built against the
# same or an older glibc, and a musl base would load nothing.
#
# RUBY_VERSION has no default on purpose. mk/sdk.mk maps RUBY_VER (3.4, 4.0)
# to the patch release, and it is the one place that number is written.
ARG RUBY_VERSION
FROM ruby:${RUBY_VERSION}-slim-bookworm

# For fiddle's C extension. The gemspec declares fiddle, so bundler builds the
# locked release instead of using the one that came with the interpreter.
RUN apt-get update \
  && apt-get install -y --no-install-recommends build-essential libffi-dev \
  && rm -rf /var/lib/apt/lists/*

# Only the files that decide the bundle, outside /repo (the mount would hide
# them). Editing lib/ or spec/ then never re-runs `bundle install`.
WORKDIR /gem
COPY Gemfile Gemfile.lock xlsx_graft.gemspec ./
COPY lib/xlsx_graft/version.rb lib/xlsx_graft/version.rb
ENV BUNDLE_GEMFILE=/gem/Gemfile
# `version system`: run the bundler that came with this interpreter rather than
# switch to the lockfile's BUNDLED WITH. One lockfile serves every gated line,
# and bundler 2.6 under Ruby 4.0 warns about redefined constants on every run.
# The interpreter is pinned to a patch release, so its bundler is pinned too.
RUN bundle config set --global frozen true \
  && bundle config set --global version system \
  && bundle install

WORKDIR /repo/sdk/ruby
