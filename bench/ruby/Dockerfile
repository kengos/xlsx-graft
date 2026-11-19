# The benchmark environment: `make bench:compare` and `make bench:cli` build
# and run this image, so the host needs no Ruby installed. The build context is
# bench/ruby.
#
# The image holds the interpreter, the competing xlsx gems and GNU time
# (/usr/bin/time -v, which reports the peak resident set size), and nothing
# else. The repository is mounted at /repo at run time: the Ruby SDK is loaded
# from /repo/sdk/ruby/lib, the engine library from the path
# XLSX_GRAFT_LIBRARY names, and the CLI from the path XLSX_GRAFT_CLI names. Changing
# the engine never rebuilds this image. The engine library and the CLI are
# built on bookworm, so this image's base is bookworm too (a musl base would
# load neither).
#
# RUBY_VERSION has no default on purpose. mk/bench.mk takes it from the latest
# line of RUBY_LINES (mk/sdk/ruby.mk), the one place that number is written.
ARG RUBY_VERSION
FROM ruby:${RUBY_VERSION}-slim-bookworm

# build-essential and libffi-dev are for the C extensions of the gems (fiddle's
# among them). `time` is GNU time, installed at /usr/bin/time.
RUN apt-get update \
  && apt-get install -y --no-install-recommends build-essential libffi-dev time \
  && rm -rf /var/lib/apt/lists/*

# Only the files that decide the bundle, outside /repo (the mount would hide
# them). Editing the scripts then never re-runs `bundle install`.
WORKDIR /bench
COPY Gemfile Gemfile.lock ./
ENV BUNDLE_GEMFILE=/bench/Gemfile
# `version system`: run the bundler that came with this interpreter rather than
# switch to the lockfile's BUNDLED WITH, as sdk/ruby/Dockerfile does.
RUN bundle config set --global frozen true \
  && bundle config set --global version system \
  && bundle install

WORKDIR /repo
