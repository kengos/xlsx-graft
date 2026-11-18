// The C ABI's view functions (engine/capi/include/xlsx_graft.h, § VIEWS),
// called through unmanaged function pointers that spell out every argument.
// Each method makes the view handle, copies its result's JSON out and closes
// the handle before it returns, so no handle outlives the call
// (docs/sdk.md § Viewing through the C ABI).
//
// The handle is a local the library writes through a pointer, and the close
// is in a `finally` that runs only for a handle that was written, as in
// VerifyCall. Every input is pinned for the call and not copied: the
// workbook's bytes too, which the library reads during the call only.
//
// For XLSX_GRAFT_ERR_REQUEST_INVALID the method reads
// xlsx_graft_error_message right after the call, on the same thread, as the
// header says.
//
// A ViewCall holds only the function pointers and does not change after it is
// made, so any thread may call a method at once. The methods are virtual so
// that a test can make one of them return a status that no workbook makes the
// engine return (PANIC, an unknown status).

namespace XlsxGraft;

/// <summary>The view functions of one engine library.</summary>
internal unsafe class ViewCall
{
    private readonly delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int> treePath;
    private readonly delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int> treeBytes;
    private readonly delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int> sheetPath;
    private readonly delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int> sheetBytes;
    private readonly delegate* unmanaged[Cdecl]<IntPtr, byte**, nuint*, int> viewResult;
    private readonly delegate* unmanaged[Cdecl]<IntPtr, int> viewClose;
    private readonly delegate* unmanaged[Cdecl]<byte**, nuint*, int> errorMessage;

    /// <summary>The functions <paramref name="library"/> exports.</summary>
    /// <exception cref="LibraryNotFoundException">The library lacks one of them.</exception>
    internal ViewCall(Library library)
    {
        treePath = (delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int>)library.Export("xlsx_graft_tree_path");
        treeBytes = (delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int>)library.Export("xlsx_graft_tree_bytes");
        sheetPath = (delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int>)library.Export("xlsx_graft_describe_sheet_path");
        sheetBytes = (delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int>)library.Export("xlsx_graft_describe_sheet_bytes");
        viewResult = (delegate* unmanaged[Cdecl]<IntPtr, byte**, nuint*, int>)library.Export("xlsx_graft_view_result");
        viewClose = (delegate* unmanaged[Cdecl]<IntPtr, int>)library.Export("xlsx_graft_view_close");
        errorMessage = (delegate* unmanaged[Cdecl]<byte**, nuint*, int>)library.Export("xlsx_graft_error_message");
    }

    /// <summary>Shows the parts of the workbook at <paramref name="path"/> (UTF-8); a tree takes no options.</summary>
    /// <param name="path">The path as UTF-8 bytes.</param>
    /// <returns>The status, and the view's JSON (its bytes copied out) when the status is OK.</returns>
    internal virtual (int Status, byte[]? Json, string? Message) TreePath(byte[] path) =>
        Made(treePath, path, []);

    /// <summary>Shows the parts of the workbook in <paramref name="data"/>, read during the call only.</summary>
    /// <param name="data">The workbook's bytes.</param>
    /// <returns>As <see cref="TreePath"/>.</returns>
    internal virtual (int Status, byte[]? Json, string? Message) TreeBytes(ReadOnlySpan<byte> data) =>
        Made(treeBytes, data, []);

    /// <summary>Shows the cells of a sheet of the workbook at <paramref name="path"/> (UTF-8).</summary>
    /// <param name="path">The path as UTF-8 bytes.</param>
    /// <param name="options">The options JSON as UTF-8 bytes.</param>
    /// <returns>
    /// The status; the view's JSON when it is OK; the engine's message when
    /// it is REQUEST_INVALID (<see langword="null"/> when it cannot be read).
    /// </returns>
    internal virtual (int Status, byte[]? Json, string? Message) SheetPath(byte[] path, byte[] options) =>
        Made(sheetPath, path, options);

    /// <summary>Shows the cells of a sheet of the workbook in <paramref name="data"/>, read during the call only.</summary>
    /// <param name="data">The workbook's bytes.</param>
    /// <param name="options">The options JSON as UTF-8 bytes.</param>
    /// <returns>As <see cref="SheetPath"/>.</returns>
    internal virtual (int Status, byte[]? Json, string? Message) SheetBytes(ReadOnlySpan<byte> data, byte[] options) =>
        Made(sheetBytes, data, options);

    private (int Status, byte[]? Json, string? Message) Made(
        delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int> function,
        ReadOnlySpan<byte> input,
        byte[] options)
    {
        var handle = IntPtr.Zero;
        try
        {
            int status;
            fixed (byte* source = input)
            fixed (byte* keys = options)
            {
                status = function(source, (nuint)input.Length, keys, (nuint)options.Length, &handle);
            }

            return status switch
            {
                StatusDiagnostic.Ok => Lent(handle),
                StatusDiagnostic.RequestInvalid => (status, null, Message()),
                _ => (status, null, null),
            };
        }
        finally
        {
            if (handle != IntPtr.Zero)
            {
                _ = viewClose(handle);
            }
        }
    }

    // The status of view_result, with the handle's JSON copied out when it is OK.
    private (int Status, byte[]? Json, string? Message) Lent(IntPtr handle)
    {
        byte* address = null;
        nuint length = 0;
        var status = viewResult(handle, &address, &length);
        return status == StatusDiagnostic.Ok
            ? (status, new ReadOnlySpan<byte>(address, checked((int)length)).ToArray(), null)
            : (status, null, null);
    }

    // Why the call that returned REQUEST_INVALID failed; null when
    // xlsx_graft_error_message does not return OK.
    private string? Message()
    {
        byte* address = null;
        nuint length = 0;
        return errorMessage(&address, &length) == StatusDiagnostic.Ok ? Capi.Text(address, length) : null;
    }
}
