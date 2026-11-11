// Verifies workbooks through the C ABI (VerifyCall.cs), under the host's
// policy and the rules of the caller. A Verifier holds only the VerifyCall
// and the policy's JSON text, which the engine checked when the verifier was
// made, and does not change after that, so it needs no lock.

using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace XlsxGraft;

/// <summary>
/// Verifies workbooks under the host's policy (given to the constructor, and
/// checked there) and the rules of the caller (given to each call).
/// </summary>
/// <remarks>
/// A verifier does not change once it is made, so one may be shared by any
/// number of threads, and their calls may run at once.
/// </remarks>
/// <example>
/// <code>
/// var verifier = new XlsxGraft.Verifier(new JsonObject { ["maxFormulaBytes"] = 32768 });
/// var result = verifier.Verify("book.xlsx", targets: ["excel >= 2016"], allowParts: ["macro"]);
/// foreach (var d in result.Diagnostics)
/// {
///     Console.Error.WriteLine($"{d.Severity} {d.Code}: {d.Message}");
/// }
/// </code>
/// </example>
public sealed class Verifier
{
    private readonly VerifyCall call;

    // The policy as JSON text, which the engine took; "null" for none.
    private readonly string policy;

    /// <summary>Makes a verifier for <paramref name="policy"/>, which the engine checks now.</summary>
    /// <param name="policy">
    /// The host's policy: the engine's limits, <c>failOn</c>,
    /// <c>denyScheduled</c> and <c>forbid</c> (<c>functions</c> and
    /// <c>parts</c>), by the engine's camelCase names, which are not changed.
    /// The same policy file as the CLI's <c>--policy</c> is
    /// <c>JsonNode.Parse(File.ReadAllText("policy.json"))!.AsObject()</c>.
    /// <see langword="null"/> is the engine's defaults. The verifier keeps
    /// the JSON of the policy at the time the constructor runs, so a later
    /// change to the <see cref="JsonObject"/> has no effect on it.
    /// </param>
    /// <param name="library">
    /// The engine library's path, used when <c>XLSX_GRAFT_LIBRARY</c> is not set.
    /// </param>
    /// <exception cref="PolicyInvalidException">The engine does not take the policy.</exception>
    /// <exception cref="ArgumentException">The policy holds a value JSON cannot write (<see cref="double.NaN"/>, a <see cref="Type"/>), or nests too deep for JSON.</exception>
    /// <exception cref="LibraryNotFoundException">No engine library was found or loaded, or it lacks a function.</exception>
    public Verifier(JsonObject? policy = null, string? library = null)
        : this(policy, library, Environment.GetEnvironmentVariable)
    {
    }

    internal Verifier(JsonObject? policy, string? library, Func<string, string?> env)
        : this(policy, new VerifyCall(Library.Open(library, env)))
    {
    }

    internal Verifier(JsonObject? policy, VerifyCall call)
    {
        this.call = call;
        this.policy = policy is null ? "null" : Checked(Json(policy));
    }

    /// <summary>Verifies the workbook at <paramref name="path"/>.</summary>
    /// <remarks>
    /// A path that is not Unicode (a lone surrogate) cannot go to the engine,
    /// so this package reads the file and verifies its bytes: a file that
    /// cannot be read is an <c>io</c> <see cref="Result.Error"/>, and the
    /// rules are not checked then.
    /// </remarks>
    /// <param name="path">The workbook's path.</param>
    /// <param name="targets">The versions the workbook must run in (<c>"excel &gt;= 2016"</c>); a problem with one is an error.</param>
    /// <param name="bestEffort">The versions it should run in; a problem with one is a warning.</param>
    /// <param name="allowFunctions">Functions refused by default to let through (without an <c>_xlfn.</c> prefix).</param>
    /// <param name="allowParts">
    /// Kinds of part refused by default to let through (<c>macro</c>,
    /// <c>external_link</c>, <c>dde</c>, <c>ole</c>, <c>activex</c>,
    /// <c>data_connection</c>, <c>web_publish</c>).
    /// </param>
    /// <param name="failOn">
    /// <c>"warn"</c> to fail on a warning too; the stricter of this and the
    /// policy's <c>failOn</c> counts.
    /// </param>
    /// <returns>
    /// A success when the engine's <c>ok</c> is true; it has no value. A file
    /// that cannot be read is an <c>io</c> <see cref="Result.Error"/>; a
    /// fault of the rules is a <c>rules_invalid</c> or <c>target_invalid</c>
    /// diagnostic, and the workbook is not opened. With <c>failOn</c>
    /// <c>"warn"</c> a result fails on a warning with no error diagnostic.
    /// </returns>
    /// <exception cref="ArgumentNullException"><paramref name="path"/> is <see langword="null"/>.</exception>
    /// <exception cref="ArgumentException">The policy and the rules make more than 2 MiB of JSON.</exception>
    public Result Verify(
        string path,
        IEnumerable<string>? targets = null,
        IEnumerable<string>? bestEffort = null,
        IEnumerable<string>? allowFunctions = null,
        IEnumerable<string>? allowParts = null,
        string? failOn = null)
    {
        ArgumentNullException.ThrowIfNull(path);
        var options = VerifyOptions.Build(policy, targets, bestEffort, allowFunctions, allowParts, failOn);
        return Capi.Utf8(path) is { } encoded ? Report(call.Path(encoded, options)) : Read(path, options);
    }

    /// <summary>
    /// Verifies the workbook in <paramref name="data"/>, which is not copied:
    /// it is pinned and read during the call only, so it must not change
    /// meanwhile.
    /// </summary>
    /// <param name="data">The workbook's bytes.</param>
    /// <param name="targets">As for <see cref="Verify"/>.</param>
    /// <param name="bestEffort">As for <see cref="Verify"/>.</param>
    /// <param name="allowFunctions">As for <see cref="Verify"/>.</param>
    /// <param name="allowParts">As for <see cref="Verify"/>.</param>
    /// <param name="failOn">As for <see cref="Verify"/>.</param>
    /// <returns>As for <see cref="Verify"/>.</returns>
    /// <exception cref="ArgumentException">As for <see cref="Verify"/>.</exception>
    public Result VerifyBytes(
        ReadOnlySpan<byte> data,
        IEnumerable<string>? targets = null,
        IEnumerable<string>? bestEffort = null,
        IEnumerable<string>? allowFunctions = null,
        IEnumerable<string>? allowParts = null,
        string? failOn = null)
    {
        var options = VerifyOptions.Build(policy, targets, bestEffort, allowFunctions, allowParts, failOn);
        return Report(call.Bytes(data, options));
    }

    private static string Json(JsonObject policy)
    {
        try
        {
            return policy.ToJsonString();
        }
        catch (Exception error) when (error is ArgumentException or InvalidOperationException or NotSupportedException or JsonException)
        {
            throw new ArgumentException($"the policy cannot be written as JSON: {error.Message}", nameof(policy), error);
        }
    }

    // The policy JSON, when the engine takes it.
    private string Checked(string json)
    {
        var result = Report(call.Policy(Encoding.UTF8.GetBytes(json)));
        return result.Success
            ? json
            : throw new PolicyInvalidException(result.Error is null ? result.Diagnostics : [result.Error, .. result.Diagnostics]);
    }

    private static Result Report((int Status, byte[]? Json) outcome) => VerifyReport.Call(outcome.Status, outcome.Json);

    // The engine takes no path that is not UTF-8, so this package reads the
    // file and gives the engine its bytes; a file that cannot be read is an
    // `io` failure, as for a file the engine cannot read.
    private Result Read(string path, byte[] options)
    {
        byte[] data;
        try
        {
            data = File.ReadAllBytes(path);
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException)
        {
            return Result.Failure([], StatusDiagnostic.Of("io", $"the file could not be read: {error.Message}"));
        }

        return Report(call.Bytes(data, options));
    }
}
