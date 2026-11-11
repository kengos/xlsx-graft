// Command check-installed-module is run by `make sdk:verify:go` after it
// builds this module, copied out of the checkout, against the SDK's copy at
// /tmp/build: it checks that the SDK came from there, alone, and that it
// reaches the engine on the gated line's Go, its version, a sheet's rows and
// the verification of bytes.
//
//	check-installed-module <the SDK's copy> <engine/Cargo.toml> <Go release>
package main

import (
	"archive/zip"
	"bytes"
	"context"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
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

	// The proxy zips the module's directory, so the license files a module
	// carries are the ones in it: copies of the repository's own.
	root := filepath.Dir(filepath.Dir(cargoToml))
	for _, name := range []string{"LICENSE-APACHE", "LICENSE-BSD", "LICENSE-MIT", "NOTICE"} {
		want, err := os.ReadFile(filepath.Join(root, name))
		check(err == nil, "%v", err)
		got, err := os.ReadFile(filepath.Join(copied, name))
		check(err == nil && bytes.Equal(got, want), "the module's %s is missing or not the repository's", name)
	}

	result, err := xlsxgraft.Version()
	check(err == nil, "Version: %v", err)
	check(result.Success, "Version failed: %+v", result.Diagnostics)
	check(result.Value == engine, "Version() is %s, the engine is %s", result.Value, engine)

	opened, err := xlsxgraft.Open(context.Background(), workbook(), func(book *xlsxgraft.Book) ([]any, error) {
		sheets, err := book.Sheets()
		if err != nil || len(sheets) != 1 {
			return nil, fmt.Errorf("sheets %+v, %w", sheets, err)
		}
		rows := book.SheetAt(0).Rows()
		defer rows.Close()
		var values []any
		for rows.Next() {
			values = append(values, rows.Row().Values...)
		}
		return append([]any{sheets[0].Name}, values...), rows.Err()
	})
	check(err == nil && opened.Success && reflect.DeepEqual(opened.Value, []any{"Data", "x"}), "Open read %+v, %v", opened, err)

	// Data that is no workbook fails with a diagnostic and no error.
	verifier, err := xlsxgraft.NewVerifier(nil)
	check(err == nil, "NewVerifier: %v", err)
	verified, err := verifier.VerifyBytes(context.Background(), nil, xlsxgraft.VerifyRules{})
	check(err == nil && !verified.Success && verified.Error == nil && len(verified.Diagnostics) > 0, "VerifyBytes gave %+v, %v", verified, err)

	fmt.Printf("%s from %s runs engine %s, reads a sheet and verifies on %s\n", sdk, copied, result.Value, runtime.Version())
}

// workbook writes a workbook of one sheet, Data, whose one cell is the
// string "x", and returns its path.
func workbook() string {
	const ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
	const relationships = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
	rels := func(id, kind, target string) string {
		return `<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="` +
			id + `" Type="` + relationships + "/" + kind + `" Target="` + target + `"/></Relationships>`
	}
	dir, err := os.MkdirTemp("", "check-installed-module-")
	check(err == nil, "%v", err)
	path := filepath.Join(dir, "book.xlsx")
	file, err := os.Create(path)
	check(err == nil, "%v", err)
	archive := zip.NewWriter(file)
	for _, entry := range [][2]string{
		{"[Content_Types].xml", `<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/></Types>`},
		{"_rels/.rels", rels("rId1", "officeDocument", "xl/workbook.xml")},
		{"xl/workbook.xml", `<workbook xmlns="` + ns + `" xmlns:r="` + relationships + `"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>`},
		{"xl/_rels/workbook.xml.rels", rels("rId1", "worksheet", "sheet1.xml")},
		{"xl/sheet1.xml", `<worksheet xmlns="` + ns + `"><sheetData><row><c t="inlineStr"><is><t>x</t></is></c></row></sheetData></worksheet>`},
	} {
		w, err := archive.Create(entry[0])
		check(err == nil, "%v", err)
		_, err = w.Write([]byte(entry[1]))
		check(err == nil, "%v", err)
	}
	check(archive.Close() == nil && file.Close() == nil, "%s cannot be written", path)
	return path
}
