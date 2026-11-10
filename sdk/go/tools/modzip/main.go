// Command modzip makes the zip the Go module proxy makes of a module, with
// golang.org/x/mod/zip, the code the proxy runs, and fails when the module
// cannot be made into one: a file name or a size it refuses, a symbolic link.
//
//	modzip <module directory> <module path> <version> <zip file>
package main

import (
	"fmt"
	"os"

	"golang.org/x/mod/module"
	modzip "golang.org/x/mod/zip"
)

func main() {
	if len(os.Args) != 5 {
		fmt.Fprintln(os.Stderr, "usage: modzip <module directory> <module path> <version> <zip file>")
		os.Exit(2)
	}
	dir, path, version, out := os.Args[1], os.Args[2], os.Args[3], os.Args[4]

	files, err := modzip.CheckDir(dir)
	if err != nil {
		fmt.Fprintf(os.Stderr, "modzip: %v\n", err)
		os.Exit(1)
	}
	if err := files.Err(); err != nil {
		fmt.Fprintf(os.Stderr, "modzip: %v\n", err)
		os.Exit(1)
	}

	file, err := os.Create(out) //nolint:gosec // The path is the caller's own.
	if err != nil {
		fmt.Fprintf(os.Stderr, "modzip: %v\n", err)
		os.Exit(1)
	}
	err = modzip.CreateFromDir(file, module.Version{Path: path, Version: version}, dir)
	if closeErr := file.Close(); err == nil {
		err = closeErr
	}
	if err != nil {
		fmt.Fprintf(os.Stderr, "modzip: %v\n", err)
		os.Exit(1)
	}
	fmt.Printf("modzip: %s: %d files, %d omitted\n", out, len(files.Valid), len(files.Omitted))
}
