namespace XlsxGraft;

/// <summary>
/// What every call into the engine returns: whether it worked, the value when
/// it did, and the diagnostics either way.
/// </summary>
/// <typeparam name="T">What the call produces.</typeparam>
public sealed class Result<T>
    where T : notnull
{
    private Result(bool success, T? value, IReadOnlyList<Diagnostic> diagnostics)
    {
        Success = success;
        Value = value;
        Diagnostics = diagnostics;
    }

    /// <summary>Whether the call worked.</summary>
    public bool Success { get; }

    /// <summary>Whether it did not.</summary>
    public bool Failed => !Success;

    /// <summary>What the call produced on a success.</summary>
    /// <remarks>
    /// On a failure it is <see langword="default"/>, which for a value type is
    /// a value of its own, so read <see cref="Success"/> first.
    /// </remarks>
    public T? Value { get; }

    /// <summary>What the engine reported, on a success as well as on a failure.</summary>
    public IReadOnlyList<Diagnostic> Diagnostics { get; }

    internal static Result<T> Ok(T value) => new(success: true, value, []);

    internal static Result<T> Failure(params Diagnostic[] diagnostics) =>
        new(success: false, default, [.. diagnostics]);
}
