namespace XlsxGraft;

/// <summary>
/// Where a relationship points, as the engine's <c>tree</c> shows it: a part
/// of the package, or a target outside it.
/// </summary>
/// <param name="Part">The part's name, or <see langword="null"/> for an external target.</param>
/// <param name="External">The external target, or <see langword="null"/> for a part.</param>
public sealed record TreeTarget(string? Part, string? External);

/// <summary>One relationship of a package or of a part.</summary>
/// <param name="Id">The relationship's id (<c>rId1</c>).</param>
/// <param name="Type">The relationship's type URI.</param>
/// <param name="Target">What it points to.</param>
public sealed record TreeRelationship(string Id, string Type, TreeTarget Target);

/// <summary>One part of a package, as the engine's <c>tree</c> shows it.</summary>
/// <param name="Name">The part's name (<c>/xl/workbook.xml</c>).</param>
/// <param name="ContentType">Its content type, or <see langword="null"/> when none is declared.</param>
/// <param name="ContentTypeKnown">Whether the content type could be determined.</param>
/// <param name="Size">Its size in bytes, uncompressed.</param>
/// <param name="CompressedSize">Its size in the archive.</param>
/// <param name="Readable">Whether the part could be read.</param>
/// <param name="Referenced">Whether a relationship points at it, or <see langword="null"/> when that is not known.</param>
/// <param name="RelationshipsKnown">Whether the part's relationships could be read.</param>
/// <param name="Relationships">The relationships the part holds.</param>
public sealed record TreePart(
    string Name,
    string? ContentType,
    bool ContentTypeKnown,
    ulong Size,
    ulong CompressedSize,
    bool Readable,
    bool? Referenced,
    bool RelationshipsKnown,
    IReadOnlyList<TreeRelationship> Relationships);

/// <summary>
/// The parts of a workbook and the relationships between them, the value of
/// <see cref="Viewer.Tree(string)"/>. Names and text are made for display
/// (control characters removed, long text cut), so they are not exact.
/// </summary>
/// <param name="Complete">Whether every part is listed.</param>
/// <param name="RelationshipsKnown">Whether the package's relationships could be read.</param>
/// <param name="Relationships">The package's own relationships.</param>
/// <param name="Parts">The parts.</param>
public sealed record Tree(
    bool Complete,
    bool RelationshipsKnown,
    IReadOnlyList<TreeRelationship> Relationships,
    IReadOnlyList<TreePart> Parts);
