// Builds the options JSON of a describe_sheet call
// (engine/capi/include/xlsx_graft.h, § VIEWS) as UTF-8 bytes, by the engine's
// names (sheet, sheetIndex, range, maxRows, maxColumns). A value the caller
// did not give (null) is not written. ViewOptions does not check a value: the
// engine does, and refuses a wrong one (REQUEST_INVALID), which ViewReport
// turns into an ArgumentException. A sheet name or a range that is not Unicode
// (a lone surrogate) cannot be written as UTF-8, and writing U+FFFD in its
// place would look for another sheet, so it is an ArgumentException before the
// C ABI is called.

using System.Buffers;
using System.Text.Json;

namespace XlsxGraft;

internal static class ViewOptions
{
    /// <summary>The most JSON the engine takes as options.</summary>
    internal const int MaxBytes = 2 << 20;

    /// <summary>The options JSON as UTF-8 bytes.</summary>
    /// <param name="sheet">The sheet's name.</param>
    /// <param name="sheetIndex">The sheet's index from 0; the engine refuses it together with a name.</param>
    /// <param name="range">An A1 range.</param>
    /// <param name="maxRows">The most rows to show.</param>
    /// <param name="maxColumns">The most columns to show.</param>
    /// <exception cref="ArgumentException">
    /// <paramref name="sheet"/> or <paramref name="range"/> holds a lone
    /// surrogate, or the options are more than 2 MiB of JSON.
    /// </exception>
    internal static byte[] Build(string? sheet, int? sheetIndex, string? range, int? maxRows, int? maxColumns)
    {
        Unicode(sheet, nameof(sheet));
        Unicode(range, nameof(range));
        var buffer = new ArrayBufferWriter<byte>();
        using (var writer = new Utf8JsonWriter(buffer))
        {
            writer.WriteStartObject();
            if (sheet is not null)
            {
                writer.WriteString("sheet", sheet);
            }

            if (sheetIndex is { } index)
            {
                writer.WriteNumber("sheetIndex", index);
            }

            if (range is not null)
            {
                writer.WriteString("range", range);
            }

            if (maxRows is { } rows)
            {
                writer.WriteNumber("maxRows", rows);
            }

            if (maxColumns is { } columns)
            {
                writer.WriteNumber("maxColumns", columns);
            }

            writer.WriteEndObject();
        }

        return buffer.WrittenCount <= MaxBytes
            ? buffer.WrittenSpan.ToArray()
            : throw new ArgumentException($"the options are {buffer.WrittenCount} bytes of JSON; the engine takes {MaxBytes}");
    }

    private static void Unicode(string? value, string name)
    {
        if (value is not null && Capi.Utf8(value) is null)
        {
            throw new ArgumentException($"{name}: the string holds a lone surrogate, which UTF-8 cannot encode", name);
        }
    }
}
