// What this package throws.
//
// A call into the engine never throws; it returns a Result. What throws is a
// setup that cannot reach the engine.

namespace XlsxGraft;

/// <summary>The root of what this package throws.</summary>
public class XlsxGraftException : Exception
{
    /// <summary>Creates the exception with <paramref name="message"/>.</summary>
    /// <param name="message">What went wrong.</param>
    public XlsxGraftException(string message)
        : base(message)
    {
    }

    /// <summary>Creates the exception with <paramref name="message"/> over <paramref name="inner"/>.</summary>
    /// <param name="message">What went wrong.</param>
    /// <param name="inner">What it went wrong with.</param>
    public XlsxGraftException(string message, Exception inner)
        : base(message, inner)
    {
    }
}

/// <summary>
/// No engine library could be found or loaded, or it lacks a function this
/// package calls.
/// </summary>
/// <remarks>
/// A configuration mistake, not a failed call, so it is thrown rather than
/// returned as a <see cref="Result{T}"/>.
/// </remarks>
public class LibraryNotFoundException : XlsxGraftException
{
    /// <inheritdoc cref="XlsxGraftException(string)"/>
    public LibraryNotFoundException(string message)
        : base(message)
    {
    }

    /// <inheritdoc cref="XlsxGraftException(string, Exception)"/>
    public LibraryNotFoundException(string message, Exception inner)
        : base(message, inner)
    {
    }
}

/// <summary>
/// The policy given to a <see cref="Verifier"/> is not one the engine takes.
/// </summary>
/// <remarks>
/// A mistake of the caller's, not a failed call, so it is thrown by the
/// constructor rather than returned as a <see cref="Result"/>. It is thrown
/// too when the engine's check of the policy itself failed (the error code is
/// <c>panic</c>, <c>null_arg</c>, <c>unknown_status</c> or
/// <c>diagnostics_unreadable</c>): the first of <see cref="Diagnostics"/> is
/// then that error.
/// </remarks>
public class PolicyInvalidException : XlsxGraftException
{
    /// <summary>Creates the exception for the <paramref name="diagnostics"/> the engine gave for the policy.</summary>
    /// <param name="diagnostics">
    /// What is wrong with the policy: the engine's error first when it gave
    /// one, then its diagnostics (a <c>policy_invalid</c> for each key).
    /// </param>
    public PolicyInvalidException(IReadOnlyList<Diagnostic> diagnostics)
        : base($"the policy is not valid: {string.Join("; ", diagnostics.Select(d => d.Message))}")
    {
        Diagnostics = diagnostics;
    }

    /// <summary>What is wrong with the policy.</summary>
    public IReadOnlyList<Diagnostic> Diagnostics { get; }
}
