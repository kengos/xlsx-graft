package xlsxgraft

// Tree is the parts of a workbook's package and the relationships between
// them, the value of [Viewer.Tree]. Names and text are made for display
// (control characters removed, long text cut), so they are not exact.
type Tree struct {
	// Complete is whether every part is listed.
	Complete bool
	// RelationshipsKnown is whether the package's relationships could be read.
	RelationshipsKnown bool
	// Relationships are the package's own relationships.
	Relationships []TreeRelationship
	// Parts are the parts, in the order the engine lists them.
	Parts []TreePart
}

// TreePart is one part of a package, as the engine's `tree` shows it.
type TreePart struct {
	// Name is the part's name (`/xl/workbook.xml`).
	Name string
	// ContentType is the part's content type, nil when none is declared.
	ContentType *string
	// ContentTypeKnown is whether the content type could be determined.
	ContentTypeKnown bool
	// Size is the part's size in bytes, uncompressed.
	Size uint64
	// CompressedSize is the part's size in the archive.
	CompressedSize uint64
	// Readable is whether the part could be read.
	Readable bool
	// Referenced is whether a relationship points at the part, nil when that
	// is not known.
	Referenced *bool
	// RelationshipsKnown is whether the part's relationships could be read.
	RelationshipsKnown bool
	// Relationships are the relationships the part holds.
	Relationships []TreeRelationship
}

// TreeRelationship is one relationship of a package or of a part.
type TreeRelationship struct {
	// ID is the relationship's id (`rId1`).
	ID string
	// Type is the relationship's type URI.
	Type string
	// Target is the part or the external target the relationship points to.
	Target TreeTarget
}

// TreeTarget is where a relationship points, as the engine's `tree` shows it:
// a part of the package, or a target outside it. One of Part and External is
// nil.
type TreeTarget struct {
	// Part is the part's name, nil for an external target.
	Part *string
	// External is the external target, nil for a part.
	External *string
}
