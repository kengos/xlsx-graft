namespace XlsxGraft;

/// <summary>One thing the engine reported about a call.</summary>
/// <param name="Code">The stable, machine-readable part.</param>
/// <param name="Message">The part for people.</param>
public sealed record Diagnostic(string Code, string Message);
