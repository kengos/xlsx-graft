// Run by `make sdk:verify:go`: a user's module of its own, built from the SDK's
// copy at /tmp/build alone, with no proxy to fetch anything from.
module check-installed-module

go 1.26

require github.com/kengos/xlsx-graft/sdk/go v0.0.0

replace github.com/kengos/xlsx-graft/sdk/go => /tmp/build
