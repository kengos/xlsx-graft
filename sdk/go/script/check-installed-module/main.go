// Command check-installed-module is run by `make sdk:verify:go` after it
// builds this module, copied out of the checkout, against the SDK's copy at
// /tmp/build: it checks that the SDK came from there, alone, and that it
// reaches the engine on the gated line's Go.
//
//	check-installed-module <the SDK's copy> <engine/Cargo.toml> <Go release>
package main

import (
	"fmt"
	"os"
	"regexp"
	"runtime"
	"runtime/debug"

	xlsxgraft "github.com/kengos/xlsx-graft/sdk/go"
)

const sdk = "github.com/kengos/xlsx-graft/sdk/go"

func check(ok bool, format string, args ...any) {
	if !ok {
		fmt.Fprintf(os.Stderr, format+"\n", args...)
		os.Exit(1)
	}
}

func main() {
	check(len(os.Args) == 4, "usage: %s <the SDK's copy> <engine/Cargo.toml> <Go release>", os.Args[0])
	copied, cargoToml, release := os.Args[1], os.Args[2], os.Args[3]

	text, err := os.ReadFile(cargoToml)
	check(err == nil, "%v", err)
	found := regexp.MustCompile(`(?ms)^\[workspace\.package\][^\[]*?^version = "([^"]+)"`).FindSubmatch(text)
	check(found != nil, "%s has no [workspace.package] version", cargoToml)
	engine := string(found[1])

	check(runtime.Version() == "go"+release, "built on %s, not on the gated go%s", runtime.Version(), release)

	// The build's own record of its modules: the SDK and nothing else, and
	// the SDK from the copy rather than from a proxy or the checkout.
	info, read := debug.ReadBuildInfo()
	check(read, "the binary carries no build info")
	check(len(info.Deps) == 1, "built with %d modules beside the main one, not %s alone", len(info.Deps), sdk)
	dep := info.Deps[0]
	check(dep.Path == sdk, "built with %s, not %s", dep.Path, sdk)
	check(dep.Replace != nil && dep.Replace.Path == copied, "built %s from %+v, not from %s", sdk, dep.Replace, copied)

	result, err := xlsxgraft.Version()
	check(err == nil, "Version: %v", err)
	check(result.Success, "Version failed: %+v", result.Diagnostics)
	check(result.Value == engine, "Version() is %s, the engine is %s", result.Value, engine)

	fmt.Printf("%s from %s runs engine %s on %s\n", sdk, copied, result.Value, runtime.Version())
}
