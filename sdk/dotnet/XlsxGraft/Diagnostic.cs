using System.Text.Json;

namespace XlsxGraft;

/// <summary>One thing the engine reported about a call.</summary>
/// <remarks>
/// The engine's own diagnostics (<see cref="Result.Diagnostics"/> of a read)
/// carry every field of the JSON every host prints. One the package makes
/// from a C ABI status is an <c>"error"</c> with <see cref="Args"/> and
/// <see cref="Location"/> empty objects and no <see cref="DecidedBy"/> or
/// <see cref="Fix"/>. Two diagnostics are equal when every field is, the JSON
/// ones compared by their content.
/// </remarks>
/// <param name="Code">The stable, machine-readable part.</param>
/// <param name="Severity"><c>"error"</c>, <c>"warning"</c> or <c>"notice"</c>.</param>
/// <param name="Message">The part for people.</param>
public sealed record Diagnostic(string Code, string Severity, string Message)
{
    // The args and location of a diagnostic the package makes: it has neither.
    private static readonly JsonElement None = JsonElement.Parse("{}");

    /// <summary>The values the message was made from, a JSON object.</summary>
    public JsonElement Args { get; init; } = None;

    /// <summary>Where in the workbook, a JSON object (its <c>part</c>, cell, ...).</summary>
    public JsonElement Location { get; init; } = None;

    /// <summary>The rule that decided it (the JSON's <c>decidedBy</c>), when the engine names one.</summary>
    public JsonElement? DecidedBy { get; init; }

    /// <summary>How to fix it, when the engine says.</summary>
    public JsonElement? Fix { get; init; }

    /// <inheritdoc/>
    public bool Equals(Diagnostic? other) =>
        other is not null
        && Code == other.Code
        && Severity == other.Severity
        && Message == other.Message
        && JsonElement.DeepEquals(Args, other.Args)
        && JsonElement.DeepEquals(Location, other.Location)
        && SameJson(DecidedBy, other.DecidedBy)
        && SameJson(Fix, other.Fix);

    /// <summary>Hashed by code, severity and message: the JSON fields have no hash of their content.</summary>
    /// <returns>The hash.</returns>
    public override int GetHashCode() => HashCode.Combine(Code, Severity, Message);

    private static bool SameJson(JsonElement? one, JsonElement? other) =>
        one is { } a && other is { } b ? JsonElement.DeepEquals(a, b) : one is null && other is null;
}
