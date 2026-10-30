// Turns the XlsxGraftValue array xlsx_graft_rows_next lends into .NET values,
// copying everything out: the array and its text live only until the next
// call on the rows handle.
//
//     kind      .NET
//     empty     null
//     string    string
//     number    double, 3.0 included
//     boolean   bool
//     date      DateOnly
//     time      TimeOnly
//     datetime  DateTime, Kind = Unspecified
//     elapsed   TimeSpan, not wrapped at a day
//     error     CellError
//
// A date, time or date-time is the workbook's wall-clock time with no time
// zone, and these types hold one as it is: none is applied.

namespace XlsxGraft;

internal static class CellValues
{
    // The kinds this package knows, from 0: XLSX_GRAFT_VALUE_ERROR is the last.
    private const uint Kinds = 9;

    /// <summary>
    /// The <paramref name="count"/> values at <paramref name="values"/>, a
    /// pointer to <c>XlsxGraftValue</c>; <see langword="null"/> when a value
    /// is of a kind this package does not know (an engine newer than the
    /// package), which is not guessed at: <paramref name="refused"/> says
    /// which.
    /// </summary>
    internal static unsafe object?[]? Call(IntPtr values, nuint count, out string? refused)
    {
        var cells = (CValue*)values;
        var row = new object?[checked((int)count)];
        for (var index = 0; index < row.Length; index++)
        {
            if (cells[index].Kind >= Kinds)
            {
                refused = $"the engine returned a value of kind {cells[index].Kind}, which this package does not know";
                return null;
            }

            row[index] = Value(in cells[index]);
        }

        refused = null;
        return row;
    }

    private static unsafe object? Value(in CValue value) => value.Kind switch
    {
        0 => null,
        1 => Capi.Text(value.Text, value.TextLen),
        2 => value.Number,
        3 => value.Boolean == 1,
        4 => Date(value),
        5 => Time(value),
        6 => Date(value).ToDateTime(Time(value), DateTimeKind.Unspecified),
        7 => TimeSpan.FromMilliseconds(checked((long)value.Elapsed)),
        _ => new CellError(Capi.Text(value.Text, value.TextLen)),
    };

    private static DateOnly Date(in CValue value) => new(value.Year, value.Month, value.Day);

    private static TimeOnly Time(in CValue value) => new(value.Hour, value.Minute, value.Second, value.Millisecond);
}
