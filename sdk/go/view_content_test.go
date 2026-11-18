package xlsxgraft

import (
	"math"
	"reflect"
	"strings"
	"testing"
)

// The JSON of `tree` and `describe-sheet` read into the types of the views
// (docs/sdk.md § Views in an SDK). Every leaf value of the two JSON objects
// below is different from every other of its kind, so that reading a key into
// another field changes a value. The flags of the three parts take true and
// false (and, for `referenced`, null), so that every flag is true somewhere
// and false somewhere, and no two flags are equal in every part.

const (
	relOne   = `{"id":"rId1","type":"type-1","target":{"part":"/target-1"}}`
	relTwo   = `{"id":"rId2","type":"type-2","target":{"external":"external-2"}}`
	relThree = `{"id":"rId3","type":"type-3","target":{"part":"/target-3","extra":0}}`
	partA    = `{"name":"/part-a","contentType":"type/a","contentTypeKnown":true,"size":120,"compressedSize":80,` +
		`"readable":false,"referenced":true,"relationshipsKnown":false,"relationships":[` + relTwo + `],"extra":[]}`
	partB = `{"name":"/part-b","contentType":null,"contentTypeKnown":false,"size":13,"compressedSize":5,` +
		`"readable":true,"referenced":null,"relationshipsKnown":false,"relationships":[` + relThree + `]}`
	partC = `{"name":"/part-c","contentType":"type/c","contentTypeKnown":false,"size":17,"compressedSize":9,` +
		`"readable":false,"referenced":false,"relationshipsKnown":true,"relationships":[]}`
)

func treeJSON(complete, known, relationships, parts string) string {
	return `{"ok":true,"error":null,"diagnostics":[],"future":1,"complete":` + complete + `,"relationshipsKnown":` + known +
		`,"relationships":[` + relationships + `],"parts":[` + parts + `]}`
}

func gridJSON(sheet, ranges, truncated, complete, rows string) string {
	return `{"ok":true,"error":null,"diagnostics":[],"future":1,"sheet":` + sheet + `,` + ranges +
		`,"truncated":` + truncated + `,"complete":` + complete + `,"rows":[` + rows + `],"merges":["A1:B1","C2:D2"]}`
}

const (
	fullRanges  = `"dimension":"A1:E9","used":"A1:D8","shown":"A1:C3"`
	emptyRanges = `"dimension":null,"used":null,"shown":null`
)

func viewedTree(t *testing.T, stdout string) *Tree {
	t.Helper()
	result, err := viewed(run{stdout: stdout}, treeView)
	if err != nil || !result.Success || result.Value == nil {
		t.Fatalf("result %+v (%v)", result, err)
	}
	return result.Value
}

func viewedGrid(t *testing.T, stdout string) *Grid {
	t.Helper()
	result, err := viewed(run{stdout: stdout}, gridView)
	if err != nil || !result.Success || result.Value == nil {
		t.Fatalf("result %+v (%v)", result, err)
	}
	return result.Value
}

func TestEveryKeyOfATreeIsReadIntoItsOwnField(t *testing.T) {
	tree := viewedTree(t, treeJSON("true", "false", relOne, partA+","+partB))

	want := &Tree{
		Complete:           true,
		RelationshipsKnown: false,
		Relationships:      []TreeRelationship{{ID: "rId1", Type: "type-1", Target: TreeTarget{Part: ptr("/target-1")}}},
		Parts: []TreePart{
			{
				Name: "/part-a", ContentType: ptr("type/a"), ContentTypeKnown: true, Size: 120, CompressedSize: 80,
				Readable: false, Referenced: ptr(true), RelationshipsKnown: false,
				Relationships: []TreeRelationship{{ID: "rId2", Type: "type-2", Target: TreeTarget{External: ptr("external-2")}}},
			},
			{
				Name: "/part-b", ContentType: nil, ContentTypeKnown: false, Size: 13, CompressedSize: 5,
				Readable: true, Referenced: nil, RelationshipsKnown: false,
				Relationships: []TreeRelationship{{ID: "rId3", Type: "type-3", Target: TreeTarget{Part: ptr("/target-3")}}},
			},
		},
	}
	if !reflect.DeepEqual(tree, want) {
		t.Errorf("tree = %+v, want %+v", tree, want)
	}
}

func TestATreeWhoseFlagsAreTheOtherWayIsReadIntoItsOwnFields(t *testing.T) {
	tree := viewedTree(t, treeJSON("false", "true", relOne, partC))

	want := &Tree{
		Complete:           false,
		RelationshipsKnown: true,
		Relationships:      []TreeRelationship{{ID: "rId1", Type: "type-1", Target: TreeTarget{Part: ptr("/target-1")}}},
		Parts: []TreePart{{
			Name: "/part-c", ContentType: ptr("type/c"), ContentTypeKnown: false, Size: 17, CompressedSize: 9,
			Readable: false, Referenced: ptr(false), RelationshipsKnown: true, Relationships: []TreeRelationship{},
		}},
	}
	if !reflect.DeepEqual(tree, want) {
		t.Errorf("tree = %+v, want %+v", tree, want)
	}
}

func TestATargetWithANullKeyIsTheSameAsOneWithoutIt(t *testing.T) {
	rel := `{"id":"a","type":"b","target":{"part":null,"external":"x"}}`

	tree := viewedTree(t, treeJSON("true", "true", rel, ""))

	if want := (TreeTarget{External: ptr("x")}); !reflect.DeepEqual(tree.Relationships[0].Target, want) {
		t.Errorf("target = %+v", tree.Relationships[0].Target)
	}
}

const (
	sheetJSON = `{"index":4,"name":"Data","kind":"dialogsheet","state":"hidden","extra":0}`
	rowsJSON  = `{"row":3,"cells":[` +
		`{"ref":"A3","type":"string","value":"text-a3","truncated":true},` +
		`{"ref":"B3","type":"boolean","value":true},` +
		`{"ref":"B4","type":"boolean","value":false,"truncated":false}]},` +
		`{"row":7,"cells":[` +
		`{"ref":"C7","type":"elapsed","value":1500},` +
		`{"ref":"D7","type":"number","value":1.5},` +
		`{"ref":"E7","type":"number","value":2.0},` +
		`{"ref":"F7","type":"date","value":"2024-01-15"},` +
		`{"ref":"G7","type":"number","value":-3},` +
		`{"ref":"H7","type":"number","value":1e3},` +
		`{"ref":"I7","type":"number","value":12345678901234567890},` +
		`{"ref":"J7","type":"number","value":1e999}]}`
)

func TestEveryKeyOfAGridIsReadIntoItsOwnField(t *testing.T) {
	grid := viewedGrid(t, gridJSON(sheetJSON, fullRanges, `{"rows":true,"columns":false,"merges":false}`, "false", rowsJSON))

	want := &Grid{
		Sheet:     &SheetInfo{Index: 4, Name: "Data", Kind: Dialogsheet, State: Hidden},
		Dimension: ptr("A1:E9"), Used: ptr("A1:D8"), Shown: ptr("A1:C3"),
		Truncated: GridTruncated{Rows: true, Columns: false, Merges: false},
		Complete:  false,
		Rows: []GridRow{
			{Row: 3, Cells: []GridCell{
				{Ref: "A3", Type: "string", Value: "text-a3", Truncated: true},
				{Ref: "B3", Type: "boolean", Value: true},
				{Ref: "B4", Type: "boolean", Value: false},
			}},
			{Row: 7, Cells: []GridCell{
				{Ref: "C7", Type: "elapsed", Value: int64(1500)},
				{Ref: "D7", Type: "number", Value: 1.5},
				{Ref: "E7", Type: "number", Value: 2.0},
				{Ref: "F7", Type: "date", Value: "2024-01-15"},
				{Ref: "G7", Type: "number", Value: int64(-3)},
				{Ref: "H7", Type: "number", Value: 1000.0},
				{Ref: "I7", Type: "number", Value: 12345678901234567890.0},
				{Ref: "J7", Type: "number", Value: math.Inf(1)},
			}},
		},
		Merges: []string{"A1:B1", "C2:D2"},
	}
	if !reflect.DeepEqual(grid, want) {
		t.Errorf("grid = %+v, want %+v", grid, want)
	}
}

func TestAGridWhoseRangesAreNullAndWhoseFlagsAreTheOtherWayIsRead(t *testing.T) {
	for name, tc := range map[string]struct {
		truncated string
		want      GridTruncated
	}{
		"columns": {`{"rows":false,"columns":true,"merges":false}`, GridTruncated{Columns: true}},
		"merges":  {`{"rows":false,"columns":false,"merges":true}`, GridTruncated{Merges: true}},
	} {
		grid := viewedGrid(t, gridJSON("null", emptyRanges, tc.truncated, "true", ""))

		want := &Grid{Truncated: tc.want, Complete: true, Rows: []GridRow{}, Merges: []string{"A1:B1", "C2:D2"}}
		if !reflect.DeepEqual(grid, want) {
			t.Errorf("%s: grid = %+v, want %+v", name, grid, want)
		}
	}
}

func TestTheKindsAndStatesAreNamedByTheirStringsAndAnotherIsUnknown(t *testing.T) {
	for kind, want := range map[string]SheetKind{
		"worksheet": Worksheet, "chartsheet": Chartsheet, "dialogsheet": Dialogsheet, "macrosheet": Macrosheet, "newer": SheetKindUnknown,
	} {
		sheet := `{"index":0,"name":"a","kind":"` + kind + `","state":"visible"}`
		if got := viewedGrid(t, gridJSON(sheet, emptyRanges, `{"rows":false,"columns":false,"merges":false}`, "true", "")).Sheet.Kind; got != want {
			t.Errorf("kind %q read as %q", kind, got)
		}
	}
	for state, want := range map[string]SheetState{
		"visible": Visible, "hidden": Hidden, "veryHidden": VeryHidden, "newer": SheetStateUnknown,
	} {
		sheet := `{"index":0,"name":"a","kind":"worksheet","state":"` + state + `"}`
		if got := viewedGrid(t, gridJSON(sheet, emptyRanges, `{"rows":false,"columns":false,"merges":false}`, "true", "")).Sheet.State; got != want {
			t.Errorf("state %q read as %q", state, got)
		}
	}
}

func TestAViewOfAnotherShapeIsUnreadable(t *testing.T) {
	flags := `{"rows":false,"columns":false,"merges":false}`
	treeWith := func(part string) string { return treeJSON("true", "true", "", part) }
	gridWith := func(sheet, rows string) string { return gridJSON(sheet, emptyRanges, flags, "true", rows) }
	sheet := strings.Replace(sheetJSON, `,"extra":0`, "", 1)
	cell := func(body string) string { return `{"row":1,"cells":[{` + body + `}]}` }
	for name, tc := range map[string]struct {
		view   string
		stdout string
	}{
		"a tree with no parts":               {"tree", `{"ok":true,"error":null,"diagnostics":[],"complete":true}`},
		"a flag that is a number":            {"tree", strings.Replace(treeWith(""), `"complete":true`, `"complete":1`, 1)},
		"relationships that are an object":   {"tree", strings.Replace(treeWith(""), `"relationships":[]`, `"relationships":{}`, 1)},
		"a part that is a number":            {"tree", treeWith("3")},
		"a relationship with no type":        {"tree", treeJSON("true", "true", `{"id":"a"}`, "")},
		"a target that is a number":          {"tree", treeJSON("true", "true", `{"id":"a","type":"t","target":3}`, "")},
		"a target part that is a number":     {"tree", treeJSON("true", "true", `{"id":"a","type":"t","target":{"part":3}}`, "")},
		"a part with a size that is text":    {"tree", treeWith(strings.Replace(partA, `"size":120`, `"size":"120"`, 1))},
		"a part with a size that is a float": {"tree", treeWith(strings.Replace(partA, `"size":120`, `"size":1.5`, 1))},
		"a part with a negative size":        {"tree", treeWith(strings.Replace(partA, `"size":120`, `"size":-1`, 1))},
		"a part with a null flag":            {"tree", treeWith(strings.Replace(partA, `"readable":false`, `"readable":null`, 1))},
		"a part with a number as referenced": {"tree", treeWith(strings.Replace(partA, `"referenced":true`, `"referenced":1`, 1))},
		"a part with a number as type":       {"tree", treeWith(strings.Replace(partA, `"contentType":"type/a"`, `"contentType":3`, 1))},
		"a grid with no keys":                {"grid", `{"ok":true,"error":null,"diagnostics":[]}`},
		"a sheet that is a number":           {"grid", gridWith("3", "")},
		"a sheet with no state":              {"grid", gridWith(`{"index":0,"name":"a","kind":"worksheet"}`, "")},
		"a sheet whose index is text":        {"grid", gridWith(strings.Replace(sheet, `"index":4`, `"index":"4"`, 1), "")},
		"a row with no cells":                {"grid", gridWith(sheet, `{"row":1}`)},
		"a cell with no value":               {"grid", gridWith(sheet, cell(`"ref":"A1","type":"string"`))},
		"a cell whose value is null":         {"grid", gridWith(sheet, cell(`"ref":"A1","type":"string","value":null`))},
		"a cell with no ref":                 {"grid", gridWith(sheet, cell(`"type":"string","value":"a"`))},
		"a cell whose truncated is a number": {"grid", gridWith(sheet, cell(`"ref":"A1","type":"string","value":"a","truncated":1`))},
		"a cell whose truncated is null":     {"grid", gridWith(sheet, cell(`"ref":"A1","type":"string","value":"a","truncated":null`))},
		"merges that hold a number":          {"grid", strings.Replace(gridWith(sheet, ""), `["A1:B1","C2:D2"]`, `[1]`, 1)},
		"truncated with no merges":           {"grid", strings.Replace(gridWith(sheet, ""), `,"merges":false}`, `}`, 1)},
	} {
		for status, want := range map[int]string{0: "invalid_output", 1: "exit_status"} {
			var result any
			var err error
			var diagnostic *Diagnostic
			if tc.view == "tree" {
				r, e := viewed(run{status: status, stdout: tc.stdout}, treeView)
				result, err, diagnostic = r, e, r.Error
				if r.Value != nil {
					t.Errorf("%s: value %+v", name, r.Value)
				}
			} else {
				r, e := viewed(run{status: status, stdout: tc.stdout}, gridView)
				result, err, diagnostic = r, e, r.Error
				if r.Value != nil {
					t.Errorf("%s: value %+v", name, r.Value)
				}
			}
			if err != nil || diagnostic == nil || diagnostic.Code != want {
				t.Errorf("%s, status %d: result %+v (%v)", name, status, result, err)
			}
		}
	}
}
