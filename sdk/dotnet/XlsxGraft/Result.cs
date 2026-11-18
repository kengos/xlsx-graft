namespace XlsxGraft;

/// <summary>
/// Whether a call worked, why it did not when it stopped, and the
/// diagnostics either way. <see cref="Book.Result"/> is one, with no value:
/// a <c>using</c> block has none to give.
/// </summary>
/// <remarks>
/// <see cref="Error"/> says why the call stopped short: the workbook or a
/// sheet asked for was refused, or the host or the engine failed. A call can
/// fail with no <see cref="Error"/> too: a read that a problem in its sheet
/// stopped, whose diagnostics hold an error, as <c>xlsx-graft read</c>'s
/// <c>ok</c> is false then.
/// </remarks>
public class Result
{
    private protected Result(bool success, IReadOnlyList<Diagnostic> diagnostics, Diagnostic? error)
    {
        Success = success;
        Diagnostics = diagnostics;
        Error = error;
    }

    /// <summary>Whether the call worked.</summary>
    public bool Success { get; }

    /// <summary>Whether it did not.</summary>
    public bool Failed => !Success;

    /// <summary>What the engine reported, on a success as well as on a failure.</summary>
    public IReadOnlyList<Diagnostic> Diagnostics { get; }

    /// <summary>The first thing that stopped the call short, or <see langword="null"/>.</summary>
    public Diagnostic? Error { get; }

    internal static Result Ok(IReadOnlyList<Diagnostic> diagnostics) => new(success: true, diagnostics, error: null);

    internal static Result Failure(IReadOnlyList<Diagnostic> diagnostics, Diagnostic? error) =>
        new(success: false, diagnostics, error);
}

/// <summary>
/// What every call into the engine that produces something returns: whether
/// it worked, the value it produced when it did (a <see cref="Viewer"/>'s
/// result has one also when it did not), and the diagnostics either way.
/// </summary>
/// <remarks>
/// A failure has no value, except one of a <see cref="Viewer"/>: its value is
/// the <see cref="Tree"/> or the <see cref="Grid"/> that could be read, also
/// when the engine's <c>ok</c> is false.
/// </remarks>
/// <typeparam name="T">What the call produces.</typeparam>
public sealed class Result<T> : Result
    where T : notnull
{
    private Result(bool success, T? value, IReadOnlyList<Diagnostic> diagnostics, Diagnostic? error = null)
        : base(success, diagnostics, error)
    {
        Value = value;
    }

    /// <summary>What the call produced on a success.</summary>
    /// <remarks>
    /// On a failure it is <see langword="default"/>, which for a value type is
    /// a value of its own, so read <see cref="Result.Success"/> first. A
    /// <see cref="Viewer"/>'s failure has a value too (a <see cref="Tree"/>
    /// or a <see cref="Grid"/>); it is <see langword="null"/> only when this
    /// package failed to get one (<c>panic</c>, <c>null_arg</c>,
    /// <c>unknown_status</c>, <c>diagnostics_unreadable</c>, or <c>io</c> for
    /// a path that is not Unicode whose file cannot be read).
    /// </remarks>
    public T? Value { get; }

    internal static Result<T> Ok(T value) => new(success: true, value, []);

    internal static Result<T> Ok(T value, IReadOnlyList<Diagnostic> diagnostics) =>
        new(success: true, value, diagnostics);

    internal static Result<T> Failure(params Diagnostic[] diagnostics) =>
        new(success: false, default, [.. diagnostics]);

    internal static Result<T> Failure(IReadOnlyList<Diagnostic> diagnostics, Diagnostic? error, T? value) =>
        new(success: false, value, diagnostics, error);
}
