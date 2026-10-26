# The PHP SDK's gate environment: `make sdk:*:php` builds and runs this image,
# so nobody needs PHP installed. The build context is sdk/php.
#
# The image holds PHP (with the zip extension), Composer, pcov and the locked
# tools, and nothing else.
# The engine is NOT copied in: the gate mounts the repository at /repo and
# points XLSX_GRAFT_CLI at the CLI `make engine:cli-bin` built there, so
# changing the engine never rebuilds this image. That CLI is built on
# bookworm, so this is bookworm too: a binary linked against glibc runs on the
# same or a newer glibc, and on a musl base not at all.
#
# PHP_PATCH has no default on purpose. mk/sdk/php.mk maps PHP_VER (8.3, …) to
# the patch release, and it is the one place that number is written.
ARG PHP_PATCH

# `sdk:lock:php` runs this stage alone: it has to make composer.lock, so it
# cannot install from it.
FROM php:${PHP_PATCH}-cli-bookworm AS tools

# The zip extension is what reads the package's zip: Composer's `artifact`
# repository opens it (mk/sdk/php.mk). unzip is what Composer unpacks a dist
# archive with; without it, it falls back on the extension and warns.
RUN apt-get update \
  && apt-get install -y --no-install-recommends libzip-dev unzip \
  && rm -rf /var/lib/apt/lists/* \
  && docker-php-ext-install zip

COPY --from=composer:2.10.3 /usr/bin/composer /usr/bin/composer

# The gate runs as root in a container of its own, which Composer would warn
# about on every command.
ENV COMPOSER_ALLOW_SUPERUSER=1 \
    COMPOSER_NO_INTERACTION=1 \
    COMPOSER_HOME=/opt/composer

FROM tools

# pcov is the coverage driver: PHPUnit writes the clover report the 100% bar
# is read from (script/coverage-gate.php). Built from the upstream tag's
# tarball, pinned by version and sha256, rather than by `pecl install`, which
# resolves the latest release on every build. To bump: change the version,
# then take the sha256 of
# https://github.com/krakjoe/pcov/archive/refs/tags/v<version>.tar.gz.
ARG PCOV_VERSION=1.0.12
ARG PCOV_SHA256=fdd07cad8e2ff42f0c9f095d84aeef11dab0fde7a008805f61883cbcb1b3f12b
RUN curl -fsSL -o /tmp/pcov.tar.gz \
      "https://github.com/krakjoe/pcov/archive/refs/tags/v${PCOV_VERSION}.tar.gz" \
  && echo "${PCOV_SHA256}  /tmp/pcov.tar.gz" | sha256sum -c - \
  && mkdir -p /usr/src/php/ext/pcov \
  && tar -xzf /tmp/pcov.tar.gz -C /usr/src/php/ext/pcov --strip-components=1 \
  && rm /tmp/pcov.tar.gz \
  && docker-php-ext-install pcov \
  && echo "pcov.enabled=1" > /usr/local/etc/php/conf.d/pcov.ini

# Only the files that decide the tools. Editing src/ or tests/ then never
# re-runs the install, and the package itself is not installed: the gate runs
# on a copy of the checkout with /pkg/vendor copied in (mk/sdk/php.mk).
# `composer install` installs what composer.lock holds. A lock older than a
# composer.json edit only warns here; `composer validate --strict` in the lint
# step is what stops it (mk/sdk/php.mk).
WORKDIR /pkg
COPY composer.json composer.lock ./
RUN composer install --no-scripts --no-progress \
  && composer clear-cache

WORKDIR /repo/sdk/php
