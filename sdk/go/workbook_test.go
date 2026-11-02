package xlsxgraft

import (
	"archive/zip"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Workbooks for the tests to read, built as a test asks: a package with a
// shared strings table of "a" and "b", styles whose cell formats 1 to 4 are a
// date, a time, a date-time and an elapsed time, and the sheets the test
// gives. The same workbook as engine/cli/tests/bin/workbook.rs builds,
// written again in Go.

const (
	mainNS = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
	relNS  = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
)

// sheetPart is one sheet of a workbook to write.
type sheetPart struct {
	name string
	// state is the state written, "" for none.
	state string
	// rel is the type of its relationship, "" when its r:id names none.
	rel string
	xml string
}

// worksheet is a worksheet named name whose sheetData holds rows.
func worksheet(name, rows string) sheetPart {
	return sheetPart{name: name, rel: relNS + "/worksheet", xml: `<worksheet xmlns="` + mainNS + `"><sheetData>` + rows + `</sheetData></worksheet>`}
}

func chartsheet(name, state string) sheetPart {
	return sheetPart{name: name, state: state, rel: relNS + "/chartsheet", xml: `<chartsheet xmlns="` + mainNS + `"/>`}
}

// unrelated is a sheet named name whose r:id names no relationship.
func unrelated(name, state string) sheetPart {
	return sheetPart{name: name, state: state}
}

func rel(id, kind, target string) string {
	return fmt.Sprintf(`<Relationship Id="%s" Type="%s" Target="%s"/>`, id, kind, target)
}

func rels(body string) string {
	return `<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">` + body + `</Relationships>`
}

// writeWorkbook writes a workbook of sheets, in order, each at
// xl/sheets/sheet<n>.xml, to a directory of the test's own, and returns its
// path.
func writeWorkbook(t *testing.T, sheets ...sheetPart) string {
	t.Helper()
	var listed strings.Builder
	related := rel("rIdS", relNS+"/sharedStrings", "sst.xml") + rel("rIdT", relNS+"/styles", "styles.xml")
	entries := [][2]string{
		{"[Content_Types].xml", `<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">` +
			`<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>` +
			`<Default Extension="xml" ContentType="application/xml"/></Types>`},
		{"_rels/.rels", rels(rel("rId1", relNS+"/officeDocument", "xl/workbook.xml"))},
		{"xl/sst.xml", `<sst xmlns="` + mainNS + `"><si><t>a</t></si><si><t>b</t></si></sst>`},
		{"xl/styles.xml", `<styleSheet xmlns="` + mainNS + `"><cellXfs><xf numFmtId="0"/><xf numFmtId="14"/>` +
			`<xf numFmtId="21"/><xf numFmtId="22"/><xf numFmtId="46"/></cellXfs></styleSheet>`},
	}
	for i, sheet := range sheets {
		n := i + 1
		state := ""
		if sheet.state != "" {
			state = fmt.Sprintf(` state="%s"`, sheet.state)
		}
		fmt.Fprintf(&listed, `<sheet name="%s" sheetId="%d"%s r:id="rId%d"/>`, sheet.name, n, state, n)
		if sheet.rel != "" {
			related += rel(fmt.Sprintf("rId%d", n), sheet.rel, fmt.Sprintf("sheets/sheet%d.xml", n))
			entries = append(entries, [2]string{fmt.Sprintf("xl/sheets/sheet%d.xml", n), sheet.xml})
		}
	}
	entries = append(entries,
		[2]string{"xl/workbook.xml", `<workbook xmlns="` + mainNS + `" xmlns:r="` + relNS + `"><sheets>` + listed.String() + `</sheets></workbook>`},
		[2]string{"xl/_rels/workbook.xml.rels", rels(related)},
	)

	path := filepath.Join(t.TempDir(), "book.xlsx")
	file, err := os.Create(path)
	if err != nil {
		t.Fatal(err)
	}
	archive := zip.NewWriter(file)
	for _, entry := range entries {
		w, err := archive.Create(entry[0])
		if err != nil {
			t.Fatal(err)
		}
		if _, err := w.Write([]byte(entry[1])); err != nil {
			t.Fatal(err)
		}
	}
	if err := archive.Close(); err != nil {
		t.Fatal(err)
	}
	if err := file.Close(); err != nil {
		t.Fatal(err)
	}
	return path
}
