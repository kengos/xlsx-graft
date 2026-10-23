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
