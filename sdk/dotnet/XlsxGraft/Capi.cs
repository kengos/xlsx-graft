// The C ABI's book and rows functions (engine/capi/include/xlsx_graft.h),
// called through unmanaged function pointers that spell out every argument.
// Each method makes one call and returns its status first, then what it
// wrote; what it wrote is only meaningful when the status is XLSX_GRAFT_OK.
//
// A handle is written through a `ref` into a slot the caller owns, so a
// handle the library made is kept even when an exception comes between the
// call and the next line; NULL is IntPtr.Zero. What the library lends (sheet
// names, the diagnostics' JSON) is copied out before the method returns. A
// row's values are not: CellValues reads them, before the next call on that
// rows handle, or its close.
//
// Every input is a byte[] of the caller's own, pinned for the call.
//
// Thread safety is the caller's: nothing here locks. The methods are virtual
// so that a test can make one of them answer what no workbook makes the
// engine answer.

using System.Runtime.InteropServices;
using System.Text;

namespace XlsxGraft;

/// <summary>The functions of one engine library.</summary>
internal unsafe class Capi
{
    private static readonly UTF8Encoding Strict = new(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);

    private readonly delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr*, int> bookOpenPath;
    private readonly delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr*, int> bookOpenBytes;
    private readonly delegate* unmanaged[Cdecl]<IntPtr, CSheet**, nuint*, int> bookSheets;
    private readonly delegate* unmanaged[Cdecl]<IntPtr, byte**, nuint*, int> bookDiagnostics;
    private readonly delegate* unmanaged[Cdecl]<IntPtr, int> bookClose;
    private readonly delegate* unmanaged[Cdecl]<IntPtr, nuint, IntPtr*, int> rowsOpen;
    private readonly delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, IntPtr*, int> rowsOpenName;
    private readonly delegate* unmanaged[Cdecl]<IntPtr, uint*, CValue**, nuint*, int> rowsNext;
    private readonly delegate* unmanaged[Cdecl]<IntPtr, int> rowsClose;
    private readonly delegate* unmanaged[Cdecl]<byte**, nuint*, int> errorMessage;

    /// <summary>The functions <paramref name="library"/> exports.</summary>
    /// <exception cref="LibraryNotFoundException">The library lacks one of them.</exception>
    internal Capi(Library library)
    {
        bookOpenPath = (delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr*, int>)library.Export("xlsx_graft_book_open_path");
        bookOpenBytes = (delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr*, int>)library.Export("xlsx_graft_book_open_bytes");
        bookSheets = (delegate* unmanaged[Cdecl]<IntPtr, CSheet**, nuint*, int>)library.Export("xlsx_graft_book_sheets");
        bookDiagnostics = (delegate* unmanaged[Cdecl]<IntPtr, byte**, nuint*, int>)library.Export("xlsx_graft_book_diagnostics");
        bookClose = (delegate* unmanaged[Cdecl]<IntPtr, int>)library.Export("xlsx_graft_book_close");
        rowsOpen = (delegate* unmanaged[Cdecl]<IntPtr, nuint, IntPtr*, int>)library.Export("xlsx_graft_rows_open");
        rowsOpenName = (delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, IntPtr*, int>)library.Export("xlsx_graft_rows_open_name");
        rowsNext = (delegate* unmanaged[Cdecl]<IntPtr, uint*, CValue**, nuint*, int>)library.Export("xlsx_graft_rows_next");
        rowsClose = (delegate* unmanaged[Cdecl]<IntPtr, int>)library.Export("xlsx_graft_rows_close");
        errorMessage = (delegate* unmanaged[Cdecl]<byte**, nuint*, int>)library.Export("xlsx_graft_error_message");
    }

    /// <summary>
    /// The UTF-8 bytes of <paramref name="text"/>; <see langword="null"/> for
    /// a string that is not Unicode (a lone surrogate), which the engine
    /// would refuse.
    /// </summary>
    internal static byte[]? Utf8(string text)
    {
        try
        {
            return Strict.GetBytes(text);
        }
        catch (EncoderFallbackException)
        {
            return null;
        }
    }

    /// <summary>The <paramref name="length"/> UTF-8 bytes at <paramref name="address"/>, copied out.</summary>
    internal static string Text(byte* address, nuint length) =>
        length == 0 ? string.Empty : Encoding.UTF8.GetString(address, checked((int)length));

    /// <summary>Writes the book handle into <paramref name="book"/>.</summary>
    internal virtual int BookOpenPath(byte[] path, ref IntPtr book)
    {
        fixed (byte* input = path)
        fixed (IntPtr* output = &book)
        {
            return bookOpenPath(input, (nuint)path.Length, output);
        }
    }

    /// <summary>Writes the book handle into <paramref name="book"/>.</summary>
    internal virtual int BookOpenBytes(byte[] data, ref IntPtr book)
    {
        fixed (byte* input = data)
        fixed (IntPtr* output = &book)
        {
            return bookOpenBytes(input, (nuint)data.Length, output);
        }
    }

    /// <summary>The status and each sheet's name, kind and state.</summary>
    internal virtual (int Status, IReadOnlyList<(string Name, uint Kind, uint State)> Sheets) BookSheets(IntPtr book)
    {
        CSheet* sheets = null;
        nuint count = 0;
        var status = bookSheets(book, &sheets, &count);
        if (status != StatusDiagnostic.Ok)
        {
            return (status, []);
        }

        var listed = new (string, uint, uint)[checked((int)count)];
        for (var index = 0; index < listed.Length; index++)
        {
            listed[index] = (Text(sheets[index].Name, sheets[index].NameLen), sheets[index].Kind, sheets[index].State);
        }

        return (status, listed);
    }

    /// <summary>The status and the diagnostics' JSON, its bytes copied out.</summary>
    internal virtual (int Status, byte[] Json) BookDiagnostics(IntPtr book)
    {
        byte* address = null;
        nuint length = 0;
        var status = bookDiagnostics(book, &address, &length);
        return status == StatusDiagnostic.Ok
            ? (status, new ReadOnlySpan<byte>(address, checked((int)length)).ToArray())
            : (status, []);
    }

    internal virtual int BookClose(IntPtr book) => bookClose(book);

    /// <summary>Writes the rows handle into <paramref name="rows"/>.</summary>
    internal virtual int RowsOpen(IntPtr book, nuint index, ref IntPtr rows)
    {
        fixed (IntPtr* output = &rows)
        {
            return rowsOpen(book, index, output);
        }
    }

    /// <summary>Writes the rows handle into <paramref name="rows"/>.</summary>
    internal virtual int RowsOpenName(IntPtr book, byte[] name, ref IntPtr rows)
    {
        fixed (byte* input = name)
        fixed (IntPtr* output = &rows)
        {
            return rowsOpenName(book, input, (nuint)name.Length, output);
        }
    }

    /// <summary>
    /// The status, the row's number (0 at the end), and its values and their
    /// count, lent until the next call on <paramref name="rows"/>.
    /// </summary>
    internal virtual (int Status, uint Number, IntPtr Values, nuint Count) RowsNext(IntPtr rows)
    {
        uint number = 0;
        CValue* values = null;
        nuint count = 0;
        var status = rowsNext(rows, &number, &values, &count);
        return status == StatusDiagnostic.Ok ? (status, number, (IntPtr)values, count) : (status, 0, IntPtr.Zero, 0);
    }

    internal virtual int RowsClose(IntPtr rows) => rowsClose(rows);

    /// <summary>
    /// Why the last call on this thread that returned <c>XLSX_GRAFT_ERR_IO</c>
    /// failed: the operating system's thread, so call it right after the call
    /// that failed.
    /// </summary>
    internal virtual string ErrorMessage()
    {
        byte* address = null;
        nuint length = 0;
        return errorMessage(&address, &length) == StatusDiagnostic.Ok ? Text(address, length) : string.Empty;
    }
}

/// <summary><c>XlsxGraftSheet</c>: 24 bytes on a 64-bit target.</summary>
[StructLayout(LayoutKind.Sequential)]
internal unsafe struct CSheet
{
    internal byte* Name;
    internal nuint NameLen;
    internal uint Kind;
    internal uint State;
}

/// <summary><c>XlsxGraftValue</c>: 48 bytes on a 64-bit target.</summary>
[StructLayout(LayoutKind.Sequential)]
internal unsafe struct CValue
{
    internal uint Kind;
    internal ushort Year;
    internal byte Month;
    internal byte Day;
    internal byte Hour;
    internal byte Minute;
    internal byte Second;
    internal byte Boolean;
    internal ushort Millisecond;
    internal ushort Reserved;
    internal double Number;
    internal ulong Elapsed;
    internal byte* Text;
    internal nuint TextLen;
}
