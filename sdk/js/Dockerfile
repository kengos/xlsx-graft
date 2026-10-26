# The JS SDK's gate environment: `make sdk:*:js` builds and runs this image,
# so nobody needs Node.js installed. The build context is sdk/js.
#
# The image holds Node.js, pnpm and the locked tools, and nothing else. The
# engine is NOT copied in: the gate mounts the repository at /repo and points
# XLSX_GRAFT_ADDON at the addon `make engine:napi` built there, so changing the
# engine never rebuilds this image. That addon is built on bookworm, so this is
# bookworm too: glibc loads an addon built against the same or an older glibc,
# and a musl base would load nothing.
#
# JS_PATCH has no default on purpose. mk/sdk/js.mk maps JS_VER (24, …) to the
# patch release, and it is the one place that number is written.
ARG JS_PATCH

# `sdk:lock:js` runs this stage alone: it has to make pnpm-lock.yaml, so it
# cannot install from it.
FROM node:${JS_PATCH}-bookworm-slim AS tools

# pnpm is pinned here and in package.json's `packageManager`, which a test
# holds equal: pnpm would otherwise fetch the version `packageManager` names.
# Installed through npm because Node.js bundles no corepack from 25 on.
RUN npm install -g --no-audit --no-fund pnpm@12.14.0

ENV NPM_CONFIG_UPDATE_NOTIFIER=false \
    NPM_CONFIG_AUDIT=false \
    NPM_CONFIG_FUND=false

FROM tools

# Only the files that decide the tools. Editing src/ or test/ then never
# re-runs the install, and the package itself is not installed: the gate runs
# on a copy of the checkout with /pkg/node_modules linked in (mk/sdk/js.mk).
# --frozen-lockfile: a package.json edit without `make sdk:lock:js` fails here
# rather than gating on the stale lock.
WORKDIR /pkg
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile

ENV PATH=/pkg/node_modules/.bin:$PATH

WORKDIR /repo/sdk/js
