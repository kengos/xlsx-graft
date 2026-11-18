/** What {@link Viewer.tree} shows: the parts of a workbook and the relationships between them. */

/**
 * Where a relationship points, as the engine's `tree` shows it: a part of the
 * package, or a target outside it. One of the two is `null`.
 */
export class TreeTarget {
  /** The part's name, or `null` for an external target. */
  readonly part: string | null;
  /** The external target, or `null` for a part. */
  readonly external: string | null;

  constructor(part: string | null, external: string | null) {
    this.part = part;
    this.external = external;
    Object.freeze(this);
  }
}

/** One relationship of a package or of a part. */
export class TreeRelationship {
  /** The relationship's id (`rId1`). */
  readonly id: string;
  /** The relationship's type URI. */
  readonly type: string;
  /** The part or the external target the relationship points to. */
  readonly target: TreeTarget;

  constructor(id: string, type: string, target: TreeTarget) {
    this.id = id;
    this.type = type;
    this.target = target;
    Object.freeze(this);
  }
}

/** One part of a package, as the engine's `tree` shows it. */
export class TreePart {
  /** The part's name (`/xl/workbook.xml`). */
  readonly name: string;
  /** The part's content type, or `null` when none is declared. */
  readonly contentType: string | null;
  /** Whether the content type could be determined. */
  readonly contentTypeKnown: boolean;
  /** The part's size in bytes, uncompressed. */
  readonly size: number;
  /** The part's size in the archive. */
  readonly compressedSize: number;
  /** Whether the part could be read. */
  readonly readable: boolean;
  /** Whether a relationship points at the part, or `null` when that is not known. */
  readonly referenced: boolean | null;
  /** Whether the part's relationships could be read. */
  readonly relationshipsKnown: boolean;
  /** The relationships the part holds. */
  readonly relationships: readonly TreeRelationship[];

  constructor(
    name: string,
    contentType: string | null,
    contentTypeKnown: boolean,
    size: number,
    compressedSize: number,
    readable: boolean,
    referenced: boolean | null,
    relationshipsKnown: boolean,
    relationships: readonly TreeRelationship[],
  ) {
    this.name = name;
    this.contentType = contentType;
    this.contentTypeKnown = contentTypeKnown;
    this.size = size;
    this.compressedSize = compressedSize;
    this.readable = readable;
    this.referenced = referenced;
    this.relationshipsKnown = relationshipsKnown;
    this.relationships = Object.freeze([...relationships]);
    Object.freeze(this);
  }
}

/**
 * The parts of a workbook and the relationships between them, the value of
 * `Viewer.tree`. Names and text are made for display (control characters
 * removed, long text cut), so they are not exact.
 */
export class Tree {
  /** Whether every part is listed. */
  readonly complete: boolean;
  /** Whether the package's relationships could be read. */
  readonly relationshipsKnown: boolean;
  /** The package's own relationships. */
  readonly relationships: readonly TreeRelationship[];
  /** The parts. */
  readonly parts: readonly TreePart[];

  constructor(
    complete: boolean,
    relationshipsKnown: boolean,
    relationships: readonly TreeRelationship[],
    parts: readonly TreePart[],
  ) {
    this.complete = complete;
    this.relationshipsKnown = relationshipsKnown;
    this.relationships = Object.freeze([...relationships]);
    this.parts = Object.freeze([...parts]);
    Object.freeze(this);
  }
}
