// Builds the options JSON of a verify call (engine/capi/include/xlsx_graft.h,
// § VERIFY): {"policy": <the checked policy or null>, "rules": {...}}, as
// UTF-8 bytes. The rules go by the engine's names (targets, bestEffort,
// functions.allow, parts.allow, failOn); one the caller did not give (null)
// is not written. VerifyOptions does not check a value: a wrong one is a
// diagnostic of the engine's (rules_invalid, target_invalid). A string that is
// not Unicode (a lone surrogate) is written with U+FFFD in its place; the
// engine checks that string as it checks any other.

using System.Buffers;
using System.Text.Json;

namespace XlsxGraft;

internal static class VerifyOptions
{
    /// <summary>The most JSON the engine takes as the policy and the rules together.</summary>
    internal const int MaxBytes = 2 << 20;

    /// <summary>The options JSON as UTF-8 bytes.</summary>
    /// <param name="policy">The policy's JSON as the engine checked it, or <c>"null"</c>.</param>
    /// <param name="targets">The versions the workbook must run in.</param>
    /// <param name="bestEffort">The versions it should run in.</param>
    /// <param name="allowFunctions">Functions refused by default to let through.</param>
    /// <param name="allowParts">Kinds of part refused by default to let through.</param>
    /// <param name="failOn"><c>"warn"</c> to fail on a warning too.</param>
    /// <exception cref="ArgumentException">The options are more than 2 MiB of JSON.</exception>
    internal static byte[] Build(
        string policy,
        IEnumerable<string>? targets,
        IEnumerable<string>? bestEffort,
        IEnumerable<string>? allowFunctions,
        IEnumerable<string>? allowParts,
        string? failOn)
    {
        var buffer = new ArrayBufferWriter<byte>();
        using (var writer = new Utf8JsonWriter(buffer))
        {
            writer.WriteStartObject();
            writer.WritePropertyName("policy");
            writer.WriteRawValue(policy, skipInputValidation: true);
            writer.WriteStartObject("rules");
            Strings(writer, "targets", targets);
            Strings(writer, "bestEffort", bestEffort);
            Allow(writer, "functions", allowFunctions);
            Allow(writer, "parts", allowParts);
            if (failOn is not null)
            {
                writer.WriteString("failOn", failOn);
            }

            writer.WriteEndObject();
            writer.WriteEndObject();
        }

        return buffer.WrittenCount <= MaxBytes
            ? buffer.WrittenSpan.ToArray()
            : throw new ArgumentException($"the policy and the rules are {buffer.WrittenCount} bytes of JSON; the engine takes {MaxBytes}");
    }

    private static void Allow(Utf8JsonWriter writer, string name, IEnumerable<string>? allow)
    {
        if (allow is null)
        {
            return;
        }

        writer.WriteStartObject(name);
        Strings(writer, "allow", allow);
        writer.WriteEndObject();
    }

    private static void Strings(Utf8JsonWriter writer, string name, IEnumerable<string>? values)
    {
        if (values is null)
        {
            return;
        }

        writer.WriteStartArray(name);
        foreach (var value in values)
        {
            writer.WriteStringValue(value);
        }

        writer.WriteEndArray();
    }
}
