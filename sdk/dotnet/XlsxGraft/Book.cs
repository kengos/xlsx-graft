// A book and its rows share state the library does not lock, so every call
// into it, closes included, is made under the book's own lock, and what it
// lends is copied out before the lock is let go.
//
// .NET raises no exception between two plain lines (Thread.Abort is gone,
// and a ThreadInterruptedException comes only while waiting, as for the
// lock, before any call), but docs/sdk.md's rules for one still hold, against an exception of
// the package's own: a handle's slot exists before the call that fills it,
// and a handle leaves the book before the call that frees it, so such an
// exception leaks a handle at worst, never frees one twice; the result is a
// close's that ran whole. There is no finalizer: a finalizer runs on a
// thread of its own, and the book is closed by its using block.

namespace XlsxGraft;

/// <summary>
/// A workbook open to read, from <see cref="XlsxReader"/>: its sheets, and
/// the rows of each through <see cref="Sheet(string)"/>.
/// </summary>
/// <remarks>
/// <para>
/// Read it in a <c>using</c> block, which closes it however the block ends,
/// and with it every sheet's rows; using it after raises
/// <see cref="ObjectDisposedException"/>. Then <see cref="Result"/> says how
/// the reading went.
/// </para>
/// <para>
/// Nothing here leaves the block early. A file that cannot be opened lists
/// no sheets, and a sheet the engine refuses (no such sheet, an unreadable
/// workbook) reads no rows; the <see cref="Result"/> fails, saying why, as
/// it does when a problem stops a sheet's rows or the host fails. A panic in
/// the engine ends every sheet of the book: none reads a row after it, and
/// the book lists no sheets. An exception the block throws is the caller's:
/// it propagates, the book closed first.
/// </para>
/// <para>One thread at a time or several: every call takes the book's lock.</para>
/// </remarks>
public sealed class Book : IDisposable
{
    private readonly Capi capi;
    private readonly Lock gate = new();

    // The rows handles open, each freed by its sheet's read or at the end.
    private readonly List<RowsSlot> rows = [];

    private IntPtr handle;
    private bool closed;

    // Why the package stopped short where the engine keeps no failure.
    private Diagnostic? failure;
    private Result? result;

    /// <summary>Opens the book: <paramref name="opening"/> writes its handle into the slot it is given.</summary>
    internal Book(Capi capi, Opening opening)
    {
        this.capi = capi;
        try
        {
            var status = opening(capi, ref handle);

            // Read only on OK, as the header says: a panic may have written
            // half of it.
            if (status != StatusDiagnostic.Ok)
            {
                handle = IntPtr.Zero;
            }

            Failed(status);
        }
        catch
        {
            Dispose();
            throw;
        }
    }

    /// <summary>Opens a book through <paramref name="capi"/>, writing its handle into <paramref name="book"/>.</summary>
    internal delegate int Opening(Capi capi, ref IntPtr book);

    /// <summary>
    /// How the reading went, once the book is closed (after its <c>using</c>
    /// block). Its error is the first failure; the engine's words for it when
    /// the engine kept it too. It has no value: keep what you read in a
    /// variable outside the block.
    /// </summary>
    /// <remarks>
    /// A close an exception cut short read no diagnostics: its result fails
    /// with <c>interrupted</c>, never a success.
    /// </remarks>
    /// <exception cref="InvalidOperationException">The book is not closed yet.</exception>
    public Result Result
    {
        get
        {
            // Locked: a close under way on another thread has not set it yet.
            lock (gate)
            {
                if (!closed)
                {
                    throw new InvalidOperationException("the result is known once the book is closed, after its using block");
                }

                return result ?? Result.Failure([], StatusDiagnostic.Of("interrupted", "an exception cut the book's close short, before it read the diagnostics"));
            }
        }
    }

    /// <summary>
    /// The sheets, in the order the workbook lists them, hidden ones
    /// included; none when the workbook cannot be read.
    /// </summary>
    /// <returns>Each sheet's index, name, kind and state.</returns>
    /// <exception cref="ObjectDisposedException">The book is closed.</exception>
    public IReadOnlyList<SheetInfo> Sheets()
    {
        IReadOnlyList<(string Name, uint Kind, uint State)> sheets;
        lock (gate)
        {
            CheckOpen();
            (var status, sheets) = capi.BookSheets(handle);
            Failed(status);
        }

        return [.. sheets.Select((sheet, index) => new SheetInfo(
            index,
            sheet.Name,
            sheet.Kind < (uint)SheetKind.Unknown ? (SheetKind)sheet.Kind : SheetKind.Unknown,
            sheet.State < (uint)SheetState.Unknown ? (SheetState)sheet.State : SheetState.Unknown))];
    }

    /// <summary>The first sheet.</summary>
    /// <inheritdoc cref="Sheet(int)"/>
    public Sheet Sheet() => Sheet(0);

    /// <summary>
    /// The sheet named <paramref name="name"/>, letter case aside. Nothing is
    /// read until <see cref="XlsxGraft.Sheet.Rows"/>. A name no sheet has is
    /// the engine's to refuse there; to look for a sheet, look in
    /// <see cref="Sheets"/>: a refused sheet fails the <see cref="Result"/>.
    /// </summary>
    /// <param name="name">The sheet's name.</param>
    /// <returns>The sheet, to read its rows.</returns>
    /// <exception cref="ObjectDisposedException">The book is closed.</exception>
    public Sheet Sheet(string name)
    {
        ArgumentNullException.ThrowIfNull(name);
        lock (gate)
        {
            CheckOpen();
        }

        return new Sheet(this, name, 0);
    }

    /// <summary>
    /// The sheet at <paramref name="index"/>, from 0, hidden sheets and
    /// chartsheets counted. Nothing is read until
    /// <see cref="XlsxGraft.Sheet.Rows"/>. An index no sheet has is the
    /// engine's to refuse there; to look for a sheet, look in
    /// <see cref="Sheets"/>: a refused sheet fails the <see cref="Result"/>.
    /// </summary>
    /// <param name="index">From 0.</param>
    /// <returns>The sheet, to read its rows.</returns>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="index"/> is below 0.</exception>
    /// <exception cref="ObjectDisposedException">The book is closed.</exception>
    public Sheet Sheet(int index)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(index);
        lock (gate)
        {
            CheckOpen();
        }

        return new Sheet(this, null, index);
    }

    /// <summary>
    /// Reads the diagnostics, then closes the book, which closes its rows,
    /// and frees what is left of each rows handle. Once: a second call does
    /// nothing. The <c>using</c> block calls it.
    /// </summary>
    public void Dispose()
    {
        lock (gate)
        {
            if (closed)
            {
                return;
            }

            closed = true;
            EngineDiagnostics.Report report;
            try
            {
                report = Report();
            }
            finally
            {
                Free();
            }

            // The last statement: a close an exception cuts short leaves no
            // result, which Result reads as `interrupted`.
            result = Outcome(report);
        }
    }

    // What Sheet calls.

    /// <summary>The rows handle's slot, <see langword="null"/> when the sheet was refused.</summary>
    internal RowsSlot? OpenRows(string? name, int index)
    {
        lock (gate)
        {
            CheckOpen();

            // Listed before the call, so an exception after it still finds
            // the handle to free; a NULL one frees nothing.
            var slot = new RowsSlot();
            rows.Add(slot);
            int status;
            if (name is null)
            {
                status = capi.RowsOpen(handle, (nuint)index, ref slot.Handle);
            }
            else
            {
                // A name that is not Unicode is refused as the engine refuses
                // one that is not UTF-8.
                var encoded = Capi.Utf8(name);
                status = encoded is null ? StatusDiagnostic.InvalidArg : capi.RowsOpenName(handle, encoded, ref slot.Handle);
            }

            Failed(status);
            if (status == StatusDiagnostic.Ok)
            {
                return slot;
            }

            rows.Remove(slot);
            return null;
        }
    }

    /// <summary>
    /// The next row; <see langword="null"/> at the end of the sheet, or when
    /// reading it stopped short, which the <see cref="Result"/> says.
    /// </summary>
    internal Row? NextRow(RowsSlot slot)
    {
        lock (gate)
        {
            CheckOpen();
            var (status, number, values, count) = capi.RowsNext(slot.Handle);
            Failed(status);
            if (status != StatusDiagnostic.Ok || number == 0)
            {
                return null;
            }

            var cells = CellValues.Call(values, count, out var refused);
            if (cells is null)
            {
                FailWith(StatusDiagnostic.Of("unknown_value", $"row {number}: {refused}"));
                return null;
            }

            return new Row((int)number, cells);
        }
    }

    /// <summary>Frees a rows handle <see cref="OpenRows"/> made, unless the book's close did.</summary>
    internal void CloseRows(RowsSlot slot)
    {
        lock (gate)
        {
            if (rows.Remove(slot))
            {
                Failed(capi.RowsClose(slot.Handle));
                slot.Handle = IntPtr.Zero;
            }
        }
    }

    /// <summary>
    /// Closes the book, which closes its rows, and frees what is left of each
    /// rows handle. Each leaves the book before its call frees it.
    /// </summary>
    private void Free()
    {
        Failed(capi.BookClose(handle));
        handle = IntPtr.Zero;
        while (rows.Count > 0)
        {
            var slot = rows[^1];
            rows.RemoveAt(rows.Count - 1);
            Failed(capi.RowsClose(slot.Handle));
        }
    }

    private void CheckOpen() => ObjectDisposedException.ThrowIf(closed, this);

    /// <summary>
    /// Keeps <paramref name="status"/> as the book's failure unless it is OK
    /// or one is kept. The host's reason is read at once: it is the thread's,
    /// and the next failure on the thread replaces it.
    /// </summary>
    private void Failed(int status)
    {
        if (status == StatusDiagnostic.Ok)
        {
            return;
        }

        var message = status == StatusDiagnostic.IO ? capi.ErrorMessage() : null;
        FailWith(StatusDiagnostic.For(status, message));
    }

    private void FailWith(Diagnostic diagnostic) => failure ??= diagnostic;

    /// <summary>
    /// The book's diagnostics; none when they cannot be read. A book that did
    /// not open has none, its handle NULL, which the library refuses.
    /// </summary>
    private EngineDiagnostics.Report Report()
    {
        var (status, json) = capi.BookDiagnostics(handle);
        if (status != StatusDiagnostic.Ok)
        {
            Failed(status);
            return EngineDiagnostics.Unread;
        }

        try
        {
            return EngineDiagnostics.Call(json);
        }
        catch (Exception error) when (EngineDiagnostics.Unreadable(error))
        {
            FailWith(StatusDiagnostic.Of("diagnostics_unreadable", $"the engine's diagnostics could not be read: {error.Message}"));
            return EngineDiagnostics.Unread;
        }
    }

    /// <summary>
    /// The package's first failure, or the engine's: the engine's words for
    /// it when the engine kept the same.
    /// </summary>
    private Result Outcome(EngineDiagnostics.Report report)
    {
        var error = report.Error;
        if (failure is not null && (error is null || failure.Code != error.Code))
        {
            error = failure;
        }

        return error is not null || report.HasErrors
            ? Result.Failure(report.Diagnostics, error)
            : Result.Ok(report.Diagnostics);
    }

    /// <summary>Where a rows handle is written, kept by the book from before the call that fills it.</summary>
    internal sealed class RowsSlot
    {
        // A field, so that the call writes the handle into it through a ref.
        internal IntPtr Handle;
    }
}
