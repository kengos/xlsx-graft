package xlsxgraft

import (
	"bytes"
	"encoding/json"
	"errors"
)

// Reads the keys of the JSON `xlsx-graft tree` and `xlsx-graft describe-sheet`
// print beside `ok`, `error` and `diagnostics` into the types of the view. A
// key missing or of another type is recorded in f.err (only the first error is
// recorded), as for the other commands; the values are not checked beyond
// that.

// treeOf is the tree o holds.
func treeOf(f *fields, o object) *Tree {
	return &Tree{
		Complete:           get[bool](f, o, "complete"),
		RelationshipsKnown: get[bool](f, o, "relationshipsKnown"),
		Relationships:      relationshipsOf(f, o),
		Parts:              partsOf(f, o),
	}
}

func partsOf(f *fields, o object) []TreePart {
	records, _ := objects(f, o, "parts")
	parts := make([]TreePart, 0, len(records))
	for _, record := range records {
		parts = append(parts, TreePart{
			Name:               get[string](f, record, "name"),
			ContentType:        nullable[string](f, record, "contentType"),
			ContentTypeKnown:   get[bool](f, record, "contentTypeKnown"),
			Size:               get[uint64](f, record, "size"),
			CompressedSize:     get[uint64](f, record, "compressedSize"),
			Readable:           get[bool](f, record, "readable"),
			Referenced:         nullable[bool](f, record, "referenced"),
			RelationshipsKnown: get[bool](f, record, "relationshipsKnown"),
			Relationships:      relationshipsOf(f, record),
		})
	}
	return parts
}

// relationshipsOf is the relationships o holds in `relationships`.
func relationshipsOf(f *fields, o object) []TreeRelationship {
	records, _ := objects(f, o, "relationships")
	relationships := make([]TreeRelationship, 0, len(records))
	for _, record := range records {
		target := get[object](f, record, "target")
		relationships = append(relationships, TreeRelationship{
			ID:   get[string](f, record, "id"),
			Type: get[string](f, record, "type"),
			Target: TreeTarget{
				Part:     present[string](f, target, "part"),
				External: present[string](f, target, "external"),
			},
		})
	}
	return relationships
}

// gridOf is the grid o holds.
func gridOf(f *fields, o object) *Grid {
	grid := &Grid{
		Sheet:     sheetOf(f, o),
		Dimension: nullable[string](f, o, "dimension"),
		Used:      nullable[string](f, o, "used"),
		Shown:     nullable[string](f, o, "shown"),
		Complete:  get[bool](f, o, "complete"),
		Merges:    get[[]string](f, o, "merges"),
	}
	flags := get[object](f, o, "truncated")
	grid.Truncated = GridTruncated{
		Rows:    get[bool](f, flags, "rows"),
		Columns: get[bool](f, flags, "columns"),
		Merges:  get[bool](f, flags, "merges"),
	}
	records, _ := objects(f, o, "rows")
	grid.Rows = make([]GridRow, 0, len(records))
	for _, record := range records {
		grid.Rows = append(grid.Rows, GridRow{
			Row:   get[int](f, record, "row"),
			Cells: cellsOf(f, record),
		})
	}
	return grid
}

// sheetOf is the sheet o holds in `sheet`, nil when it is null.
func sheetOf(f *fields, o object) *SheetInfo {
	if raw, found := o["sheet"]; found && isNull(raw) {
		return nil
	}
	info := sheetInfoOf(f, get[object](f, o, "sheet"))
	return &info
}

func cellsOf(f *fields, row object) []GridCell {
	records, _ := objects(f, row, "cells")
	cells := make([]GridCell, 0, len(records))
	for _, record := range records {
		cell := GridCell{
			Ref:   get[string](f, record, "ref"),
			Type:  get[string](f, record, "type"),
			Value: valueOf(f, record),
		}
		// The key is there only when the text was cut.
		if _, found := record["truncated"]; found {
			cell.Truncated = get[bool](f, record, "truncated")
		}
		cells = append(cells, cell)
	}
	return cells
}

// valueOf is the cell's `value`, as the JSON writes it: a string, a bool, an
// int64 for a number written as an integer that fits in an int64, a float64
// for any other number.
func valueOf(f *fields, cell object) any {
	has(f, cell, "value")
	if isNull(cell["value"]) {
		f.fail(errors.New(`"value" is null`))
	}
	decoder := json.NewDecoder(bytes.NewReader(cell["value"]))
	decoder.UseNumber()
	var value any
	// The JSON was read once already, so it holds one value. When the cell has
	// no "value" key, the call to has recorded that in f.err, and Decode reads
	// nothing from the empty input.
	_ = decoder.Decode(&value)
	number, isNumber := value.(json.Number)
	if !isNumber {
		return value
	}
	if whole, err := number.Int64(); err == nil {
		return whole
	}
	// For a number too large for a float64, Float64 returns +Inf or -Inf with
	// strconv.ErrRange; the value is kept and the error is not.
	float, _ := number.Float64()
	return float
}

// nullable is o's key, which o must have, as a pointer to a T; nil when null.
func nullable[T any](f *fields, o object, key string) *T {
	if raw, found := o[key]; found && isNull(raw) {
		return nil
	}
	value := get[T](f, o, key)
	return &value
}

// present is o's key as a pointer to a T; nil when o has no such key or its
// value is null.
func present[T any](f *fields, o object, key string) *T {
	if _, found := o[key]; !found {
		return nil
	}
	return nullable[T](f, o, key)
}
