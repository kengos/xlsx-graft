// Calls xlsx_graft_version and returns the engine's version as a Result.
//
// The signature is written from engine/capi/include/xlsx_graft.h:
//
//     int32_t xlsx_graft_version(const uint8_t **out_ptr, size_t *out_len);
//
// `size_t` is `nuint`, not `ulong`: the two agree on 64-bit platforms only,
// and an out-parameter read at the wrong width fails silently. No
// `bool` crosses the boundary, so no marshalling guess is ever made.

using System.Text;

namespace XlsxGraft;

internal static class VersionCall
{
    internal const string Name = "xlsx_graft_version";

    /// <summary>The version string, such as <c>"0.0.0"</c>, on success.</summary>
    /// <param name="function">The address of <c>xlsx_graft_version</c> (or of a function of its type).</param>
    internal static unsafe Result<string> Call(IntPtr function)
    {
        byte* outPtr = null;
        nuint outLen = 0;
        var status = ((delegate* unmanaged[Cdecl]<byte**, nuint*, int>)function)(&outPtr, &outLen);
        if (status != StatusDiagnostic.Ok)
        {
            return Result<string>.Failure(StatusDiagnostic.For(status));
        }

        // The bytes are not NUL-terminated, so exactly `outLen` of them are
        // copied out. The header guarantees the length; one past int.MaxValue
        // throws OverflowException rather than reading out of range.
        return Result<string>.Ok(Encoding.UTF8.GetString(new ReadOnlySpan<byte>(outPtr, checked((int)outLen))));
    }
}
