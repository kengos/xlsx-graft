// Reads the keys of a view's JSON after "ok", "error" and "diagnostics" into
// a Tree or a Grid. A key it does not know is ignored. A value of the wrong
// type or a missing key throws what EngineDiagnostics.Unreadable names (or a
// FormatException, for a number out of range), which ViewReport turns into a
// diagnostics_unreadable failure; nothing here checks a value beforehand.

using System.Text.Json;

namespace XlsxGraft;

internal static class ViewContent
{
    /// <summary>The <see cref="XlsxGraft.Tree"/> that <paramref name="view"/> holds.</summary>
    /// <param name="view">The parsed JSON of a <c>tree</c> view.</param>
    /// <exception cref="Exception">For JSON of another shape: <see cref="KeyNotFoundException"/>, <see cref="InvalidOperationException"/> or <see cref="FormatException"/>.</exception>
    internal static Tree Tree(JsonElement view) =>
        new(
            view.GetProperty("complete").GetBoolean(),
            view.GetProperty("relationshipsKnown").GetBoolean(),
            Relationships(view.GetProperty("relationships")),
            [.. view.GetProperty("parts").EnumerateArray().Select(Part)]);

    /// <summary>The <see cref="XlsxGraft.Grid"/> that <paramref name="view"/> holds.</summary>
    /// <param name="view">The parsed JSON of a <c>describe_sheet</c> view.</param>
    /// <exception cref="Exception">As for <see cref="Tree"/>.</exception>
    internal static Grid Grid(JsonElement view)
    {
        var truncated = view.GetProperty("truncated");
        return new Grid(
            Sheet(view.GetProperty("sheet")),
            Text(view.GetProperty("dimension")),
            Text(view.GetProperty("used")),
            Text(view.GetProperty("shown")),
            new GridTruncated(
                truncated.GetProperty("rows").GetBoolean(),
                truncated.GetProperty("columns").GetBoolean(),
                truncated.GetProperty("merges").GetBoolean()),
            view.GetProperty("complete").GetBoolean(),
            [.. view.GetProperty("rows").EnumerateArray().Select(Row)],
            [.. view.GetProperty("merges").EnumerateArray().Select(merge => merge.GetString() ?? throw Null())]);
    }

    private static TreePart Part(JsonElement part) =>
        new(
            part.GetProperty("name").GetString() ?? throw Null(),
            Text(part.GetProperty("contentType")),
            part.GetProperty("contentTypeKnown").GetBoolean(),
            part.GetProperty("size").GetUInt64(),
            part.GetProperty("compressedSize").GetUInt64(),
            part.GetProperty("readable").GetBoolean(),
            part.GetProperty("referenced") is { ValueKind: not JsonValueKind.Null } referenced ? referenced.GetBoolean() : null,
            part.GetProperty("relationshipsKnown").GetBoolean(),
            Relationships(part.GetProperty("relationships")));

    private static List<TreeRelationship> Relationships(JsonElement records) =>
        [.. records.EnumerateArray().Select(record => new TreeRelationship(
            record.GetProperty("id").GetString() ?? throw Null(),
            record.GetProperty("type").GetString() ?? throw Null(),
            Target(record.GetProperty("target"))))];

    // "part" or "external" is absent when the target is not that kind.
    private static TreeTarget Target(JsonElement target) =>
        new(
            target.TryGetProperty("part", out var part) ? Text(part) : null,
            target.TryGetProperty("external", out var external) ? Text(external) : null);

    private static SheetInfo? Sheet(JsonElement sheet) =>
        sheet.ValueKind == JsonValueKind.Null
            ? null
            : new SheetInfo(
                sheet.GetProperty("index").GetInt32(),
                sheet.GetProperty("name").GetString() ?? throw Null(),
                sheet.GetProperty("kind").GetString() switch
                {
                    "worksheet" => SheetKind.Worksheet,
                    "chartsheet" => SheetKind.Chartsheet,
                    "dialogsheet" => SheetKind.Dialogsheet,
                    "macrosheet" => SheetKind.Macrosheet,
                    _ => SheetKind.Unknown,
                },
                sheet.GetProperty("state").GetString() switch
                {
                    "visible" => SheetState.Visible,
                    "hidden" => SheetState.Hidden,
                    "veryHidden" => SheetState.VeryHidden,
                    _ => SheetState.Unknown,
                });

    private static GridRow Row(JsonElement row) =>
        new(row.GetProperty("row").GetInt32(), [.. row.GetProperty("cells").EnumerateArray().Select(Cell)]);

    private static GridCell Cell(JsonElement cell) =>
        new(
            cell.GetProperty("ref").GetString() ?? throw Null(),
            cell.GetProperty("type").GetString() ?? throw Null(),
            Value(cell.GetProperty("value")),
            cell.TryGetProperty("truncated", out var truncated) && truncated.GetBoolean());

    // A number written as an integer is a long, any other number a double,
    // as Ruby and Python read the same JSON. The conditional's branches are
    // cast to object: without it C# makes both a double.
    private static object Value(JsonElement value) =>
        value.ValueKind switch
        {
            JsonValueKind.String => value.GetString()!,
            JsonValueKind.True => true,
            JsonValueKind.False => false,
            JsonValueKind.Number => value.TryGetInt64(out var integer) ? (object)integer : value.GetDouble(),
            _ => throw new InvalidOperationException($"a cell's value is {value.ValueKind}"),
        };

    private static string? Text(JsonElement value) =>
        value.ValueKind == JsonValueKind.Null ? null : value.GetString();

    private static InvalidOperationException Null() => new("a string is null");
}
