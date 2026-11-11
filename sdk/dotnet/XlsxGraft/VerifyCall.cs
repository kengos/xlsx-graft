// The C ABI's verify functions (engine/capi/include/xlsx_graft.h, § VERIFY),
// called through unmanaged function pointers that spell out every argument.
// Each method makes the verify handle, copies its result's JSON out and
// closes the handle before it returns, so no handle outlives the call
// (docs/sdk.md § Verifying through the C ABI).
//
// The handle is a local the library writes through a pointer, and the close
// is in a `finally` that runs only for a handle that was written. A call that
// returns PANIC, NULL_ARG or INVALID_ARG writes no handle, so it stays NULL
// (IntPtr.Zero) and nothing is closed.
//
// Every input is pinned for the call and not copied: the workbook's bytes
// too, which the library reads during the call only.
//
// A VerifyCall holds only the function pointers and does not change after it
// is made, so any thread may call a method at once. The methods are virtual so
// that a test can make one of them return a status that no workbook makes the
// engine return (PANIC, an unknown status).

namespace XlsxGraft;

/// <summary>The verify functions of one engine library.</summary>
internal unsafe class VerifyCall
{
    private readonly delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int> verifyPath;
    private readonly delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int> verifyBytes;
    private readonly delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr*, int> policyCheck;
    private readonly delegate* unmanaged[Cdecl]<IntPtr, byte**, nuint*, int> verifyResult;
    private readonly delegate* unmanaged[Cdecl]<IntPtr, int> verifyClose;

    /// <summary>The functions <paramref name="library"/> exports.</summary>
    /// <exception cref="LibraryNotFoundException">The library lacks one of them.</exception>
    internal VerifyCall(Library library)
    {
        verifyPath = (delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int>)library.Export("xlsx_graft_verify_path");
        verifyBytes = (delegate* unmanaged[Cdecl]<byte*, nuint, byte*, nuint, IntPtr*, int>)library.Export("xlsx_graft_verify_bytes");
        policyCheck = (delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr*, int>)library.Export("xlsx_graft_policy_check");
        verifyResult = (delegate* unmanaged[Cdecl]<IntPtr, byte**, nuint*, int>)library.Export("xlsx_graft_verify_result");
        verifyClose = (delegate* unmanaged[Cdecl]<IntPtr, int>)library.Export("xlsx_graft_verify_close");
    }

    /// <summary>Verifies the workbook at <paramref name="path"/> (UTF-8) under <paramref name="options"/>.</summary>
    /// <param name="path">The path as UTF-8 bytes.</param>
    /// <param name="options">The options JSON, <c>{"policy": ..., "rules": ...}</c>, as UTF-8 bytes.</param>
    /// <returns>The status, and the result's JSON (its bytes copied out) when the status is OK.</returns>
    internal virtual (int Status, byte[]? Json) Path(byte[] path, byte[] options)
    {
        var handle = IntPtr.Zero;
        try
        {
            int status;
            fixed (byte* input = path)
            fixed (byte* rules = options)
            {
                status = verifyPath(input, (nuint)path.Length, rules, (nuint)options.Length, &handle);
            }

            return Lent(status, handle);
        }
        finally
        {
            Close(handle);
        }
    }

    /// <summary>Verifies the workbook in <paramref name="data"/>, read during the call only, under <paramref name="options"/>.</summary>
    /// <param name="data">The workbook's bytes.</param>
    /// <param name="options">The options JSON, as for <see cref="Path"/>.</param>
    /// <returns>As <see cref="Path"/>.</returns>
    internal virtual (int Status, byte[]? Json) Bytes(ReadOnlySpan<byte> data, byte[] options)
    {
        var handle = IntPtr.Zero;
        try
        {
            int status;
            fixed (byte* input = data)
            fixed (byte* rules = options)
            {
                status = verifyBytes(input, (nuint)data.Length, rules, (nuint)options.Length, &handle);
            }

            return Lent(status, handle);
        }
        finally
        {
            Close(handle);
        }
    }

    /// <summary>Checks a policy alone.</summary>
    /// <param name="policy">The policy's JSON as UTF-8 bytes, not wrapped in <c>{"policy": ...}</c>.</param>
    /// <returns>As <see cref="Path"/>.</returns>
    internal virtual (int Status, byte[]? Json) Policy(byte[] policy)
    {
        var handle = IntPtr.Zero;
        try
        {
            int status;
            fixed (byte* input = policy)
            {
                status = policyCheck(input, (nuint)policy.Length, &handle);
            }

            return Lent(status, handle);
        }
        finally
        {
            Close(handle);
        }
    }

    // The status of the call, or of verify_result when the call's was OK, with
    // the result's bytes copied out.
    private (int Status, byte[]? Json) Lent(int status, IntPtr handle)
    {
        if (status != StatusDiagnostic.Ok)
        {
            return (status, null);
        }

        byte* address = null;
        nuint length = 0;
        status = verifyResult(handle, &address, &length);
        return status == StatusDiagnostic.Ok ? (status, new ReadOnlySpan<byte>(address, checked((int)length)).ToArray()) : (status, null);
    }

    private void Close(IntPtr handle)
    {
        if (handle != IntPtr.Zero)
        {
            _ = verifyClose(handle);
        }
    }
}
