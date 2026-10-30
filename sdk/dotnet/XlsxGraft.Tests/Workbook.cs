// Workbooks for the tests to read, built as a test asks: a zip holding a
// package with a shared strings table of "a" and "b", styles whose cell
// formats 1 to 4 are a date, a time, a date-time and an elapsed time, and the
// sheets the test gives. The same workbook as engine/capi/src/tests/workbook.rs
// builds, written again in C#.

using System.IO.Compression;
using System.Text;

namespace XlsxGraft.Tests;

internal static class Workbook
{
    private const string Ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    private const string Types = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    private const string Rels = "http://schemas.openxmlformats.org/package/2006/relationships";
    private const string ContentTypes =
        "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">"
        + "<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>"
        + "<Default Extension=\"xml\" ContentType=\"application/xml\"/></Types>";

    // The number formats of cell formats 0 to 4: general, a date, a time, a date-time, an elapsed time.
    private static readonly int[] Formats = [0, 14, 21, 22, 46];

    /// <summary>A worksheet named <paramref name="name"/> whose sheetData holds <paramref name="rows"/>.</summary>
    internal static Sheet Worksheet(string name, string rows = "", string? state = null) =>
        new(name, state, $"{Types}/worksheet", $"<worksheet xmlns=\"{Ns}\"><sheetData>{rows}</sheetData></worksheet>");

    internal static Sheet Chartsheet(string name) => new(name, null, $"{Types}/chartsheet", $"<chartsheet xmlns=\"{Ns}\"/>");

    /// <summary>A sheet named <paramref name="name"/> whose r:id names no relationship.</summary>
    internal static Sheet Unrelated(string name) => new(name, null, null, string.Empty);

    /// <summary>The bytes of a workbook of <paramref name="sheets"/>, in order, each at xl/sheets/sheet&lt;n&gt;.xml.</summary>
    internal static byte[] Xlsx(params Sheet[] sheets)
    {
        var numbered = sheets.Select((sheet, index) => (N: index + 1, Sheet: sheet)).ToList();
        var listed = string.Concat(numbered.Select(s =>
            $"<sheet name=\"{s.Sheet.Name}\" sheetId=\"{s.N}\""
            + (s.Sheet.State is null ? string.Empty : $" state=\"{s.Sheet.State}\"")
            + $" r:id=\"rId{s.N}\"/>"));
        var related = string.Concat(numbered
            .Where(s => s.Sheet.RelType is not null)
            .Select(s => Rel($"rId{s.N}", s.Sheet.RelType!, $"sheets/sheet{s.N}.xml")));
        var formats = string.Concat(Formats.Select(id => $"<xf numFmtId=\"{id}\"/>"));

        var entries = new List<(string, string)>
        {
            ("[Content_Types].xml", ContentTypes),
            ("_rels/.rels", Relationships(Rel("rId1", $"{Types}/officeDocument", "xl/workbook.xml"))),
            ("xl/workbook.xml", $"<workbook xmlns=\"{Ns}\" xmlns:r=\"{Types}\"><sheets>{listed}</sheets></workbook>"),
            ("xl/_rels/workbook.xml.rels", Relationships(Rel("rIdS", $"{Types}/sharedStrings", "sst.xml") + Rel("rIdT", $"{Types}/styles", "styles.xml") + related)),
            ("xl/sst.xml", $"<sst xmlns=\"{Ns}\"><si><t>a</t></si><si><t>b</t></si></sst>"),
            ("xl/styles.xml", $"<styleSheet xmlns=\"{Ns}\"><cellXfs>{formats}</cellXfs></styleSheet>"),
        };
        entries.AddRange(numbered.Where(s => s.Sheet.RelType is not null).Select(s => ($"xl/sheets/sheet{s.N}.xml", s.Sheet.Xml)));
        return Zip(entries);
    }

    private static string Rel(string id, string type, string target) =>
        $"<Relationship Id=\"{id}\" Type=\"{type}\" Target=\"{target}\"/>";

    private static string Relationships(string body) => $"<Relationships xmlns=\"{Rels}\">{body}</Relationships>";

    private static byte[] Zip(List<(string Name, string Text)> entries)
    {
        using var stream = new MemoryStream();
        using (var zip = new ZipArchive(stream, ZipArchiveMode.Create, leaveOpen: true))
        {
            foreach (var (name, text) in entries)
            {
                using var entry = zip.CreateEntry(name, CompressionLevel.NoCompression).Open();
                entry.Write(Encoding.UTF8.GetBytes(text));
            }
        }

        return stream.ToArray();
    }

    /// <summary>A sheet: its name, its state (null: none written), the type of its relationship (null: its r:id names none) and its part's XML.</summary>
    internal sealed record Sheet(string Name, string? State, string? RelType, string Xml);
}
