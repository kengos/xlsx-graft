using System.Text;
using System.Text.Json.Nodes;

namespace XlsxGraft.Tests;

public sealed class VerifierTests
{
    private static readonly Verifier Default = new(library: Repo.EngineLibrary);
    private static readonly string Macro = VerifyCallTests.Fixture("macro.xlsm");
    private static readonly string Table = VerifyCallTests.Fixture("table.xlsx");
    private static readonly string Dynamic = VerifyCallTests.Fixture("dynamic-array.xlsx");

    [Fact]
    public void Verify_fails_a_workbook_the_engine_refuses_with_its_diagnostics_and_no_error()
    {
        var result = Default.Verify(Macro);

        Assert.False(result.Success);
        Assert.Null(result.Error);
        Assert.Equal(["part_denied", "part_denied"], result.Diagnostics.Select(d => d.Code));
        var first = result.Diagnostics[0];
        Assert.Equal("error", first.Severity);
        Assert.NotNull(first.DecidedBy);
        Assert.Equal("template", first.Fix?.GetProperty("in").GetString());
    }

    [Fact]
    public void Verify_lets_through_a_part_that_is_allowed()
    {
        var allowed = Default.Verify(Macro, allowParts: ["macro"]);
        var table = Default.Verify(Table);

        Assert.True(allowed.Success);
        Assert.Empty(allowed.Diagnostics);
        Assert.True(table.Success);
        Assert.Empty(table.Diagnostics);
    }

    [Fact]
    public void Verify_fails_with_an_io_error_for_a_file_that_cannot_be_read()
    {
        var result = Default.Verify("/nowhere/book.xlsx");

        Assert.False(result.Success);
        Assert.Empty(result.Diagnostics);
        Assert.Equal("io", result.Error?.Code);
        Assert.Contains("/nowhere/book.xlsx", result.Error?.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Verify_reports_a_fault_of_the_rules_as_diagnostics_and_does_not_open_the_workbook()
    {
        var result = Default.Verify("/nowhere/book.xlsx", targets: ["excel 1990"], allowFunctions: ["_xlfn.SEQUENCE"]);

        Assert.False(result.Success);
        Assert.Null(result.Error);
        Assert.Equal(2, result.Diagnostics.Count);
        Assert.Contains("target_invalid", result.Diagnostics.Select(d => d.Code));
        var invalid = result.Diagnostics.Single(d => d.Code == "rules_invalid");
        Assert.Equal("prefixed", invalid.Args.GetProperty("reason").GetString());
    }

    [Fact]
    public void Verify_fails_on_a_warning_only_when_failOn_says_so()
    {
        var lenient = Default.Verify(Dynamic, bestEffort: ["excel 2016"]);
        var strict = Default.Verify(Dynamic, bestEffort: ["excel 2016"], failOn: "warn");

        Assert.True(lenient.Success);
        Assert.Contains("warn", lenient.Diagnostics.Select(d => d.Severity));
        Assert.False(strict.Success);
        Assert.Null(strict.Error);
        Assert.DoesNotContain("error", strict.Diagnostics.Select(d => d.Severity));
    }

    [Fact]
    public void Verify_reads_a_file_whose_path_is_not_Unicode_and_verifies_its_bytes()
    {
        // File.Copy and File.ReadAllBytes both replace each lone surrogate of
        // the path with U+FFFD, so the file copied to this path is the file
        // that Verify reads for the same path.
        var directory = Directory.CreateTempSubdirectory("xlsx-graft-");
        try
        {
            var path = Path.Combine(directory.FullName, "\uD800.xlsx");
            File.Copy(Table, path);

            var result = Default.Verify(path);

            Assert.True(result.Success);
        }
        finally
        {
            directory.Delete(recursive: true);
        }
    }

    [Fact]
    public void Verify_fails_with_an_io_error_for_a_path_that_is_not_Unicode_and_cannot_be_read()
    {
        var result = Default.Verify("/nowhere/\uD800.xlsx", targets: ["excel 1990"]);

        Assert.False(result.Success);
        Assert.Empty(result.Diagnostics);
        Assert.Equal("io", result.Error?.Code);
        Assert.StartsWith("the file could not be read: ", result.Error?.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Verify_throws_for_a_path_that_is_null()
    {
        Assert.Throws<ArgumentNullException>(() => Default.Verify(null!));
    }

    [Fact]
    public void A_rule_that_is_not_Unicode_is_reported_by_the_engine_as_target_invalid()
    {
        var result = Default.Verify(Table, targets: ["\uD800"]);

        Assert.False(result.Success);
        Assert.Equal(["target_invalid"], result.Diagnostics.Select(d => d.Code));
    }

    [Fact]
    public void Options_of_exactly_2_MiB_go_to_the_engine_and_one_byte_more_throw_ArgumentException()
    {
        // The options JSON is 2 MiB when the allowed function's name is this long.
        var overhead = VerifyOptions.Build("null", null, null, ["A"], null, null).Length - 1;
        var name = new string('A', VerifyOptions.MaxBytes - overhead);

        var exact = Default.Verify(Table, allowFunctions: [name]);

        Assert.True(exact.Success);
        Assert.Empty(exact.Diagnostics);
        Assert.Throws<ArgumentException>(() => Default.Verify(Table, allowFunctions: [name + "A"]));
    }

    [Fact]
    public void Options_of_more_than_2_MiB_throw_ArgumentException()
    {
        var error = Assert.Throws<ArgumentException>(() => Default.Verify(Table, allowFunctions: [new string('A', 2 << 20)]));

        Assert.StartsWith("the policy and the rules are ", error.Message, StringComparison.Ordinal);
        Assert.EndsWith(" bytes of JSON; the engine takes 2097152", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void VerifyBytes_verifies_the_bytes_and_bytes_that_are_not_a_workbook_fail_with_a_diagnostic()
    {
        var table = Default.VerifyBytes(File.ReadAllBytes(Table));
        var empty = Default.VerifyBytes([]);

        Assert.True(table.Success);
        Assert.False(empty.Success);
        Assert.Null(empty.Error);
        Assert.NotEmpty(empty.Diagnostics);
    }

    [Fact]
    public void VerifyBytes_takes_the_rules_of_the_call()
    {
        var result = Default.VerifyBytes(File.ReadAllBytes(Macro), allowParts: ["macro"]);

        Assert.True(result.Success);
    }

    [Fact]
    public async Task One_verifier_may_be_shared_by_threads_that_call_it_at_once()
    {
        var bytes = await File.ReadAllBytesAsync(Macro, TestContext.Current.CancellationToken);
        var tasks = Enumerable.Range(0, 4).Select(_ => Task.Run(() => Default.VerifyBytes(bytes).Diagnostics.Select(d => d.Code).ToList()));

        var results = await Task.WhenAll(tasks);

        Assert.All(results, codes => Assert.Equal(["part_denied", "part_denied"], codes));
    }

    [Fact]
    public void The_policy_is_written_to_the_engine_with_its_keys_as_they_are()
    {
        var result = new Verifier(new JsonObject { ["maxEntries"] = 1 }, Repo.EngineLibrary).Verify(Table);
        var snake = Assert.Throws<PolicyInvalidException>(() => new Verifier(new JsonObject { ["max_entries"] = 1 }, Repo.EngineLibrary));

        Assert.False(result.Success);
        Assert.Equal("policy", result.Diagnostics[0].DecidedBy?.GetProperty("source").GetString());
        Assert.Equal("max_entries", Assert.Single(snake.Diagnostics).Args.GetProperty("key").GetString());
    }

    [Fact]
    public void A_policy_parsed_by_JsonNode_Parse_is_taken_like_one_built_in_code()
    {
        var policy = JsonNode.Parse("{\"maxEntries\": 1}")!.AsObject();

        Assert.False(new Verifier(policy, Repo.EngineLibrary).Verify(Table).Success);
    }

    [Fact]
    public void The_policys_failOn_makes_a_warning_fail()
    {
        var verifier = new Verifier(new JsonObject { ["failOn"] = "warn" }, Repo.EngineLibrary);

        Assert.False(verifier.Verify(Dynamic, bestEffort: ["excel 2016"]).Success);
    }

    [Fact]
    public void The_verifier_keeps_the_policy_it_checked_when_the_caller_changes_theirs()
    {
        var policy = new JsonObject { ["maxEntries"] = 1 };
        var verifier = new Verifier(policy, Repo.EngineLibrary);
        policy["maxEntries"] = 10_000;

        Assert.False(verifier.Verify(Table).Success);
    }

    [Fact]
    public void A_policy_the_engine_does_not_take_throws_PolicyInvalidException_with_the_engines_diagnostics()
    {
        var policy = JsonNode.Parse("{\"maxEntries\":0,\"forbid\":{\"parts\":[{\"kind\":\"nope\"}]}}")!.AsObject();

        var error = Assert.Throws<PolicyInvalidException>(() => new Verifier(policy, Repo.EngineLibrary));

        Assert.All(error.Diagnostics, d => Assert.Equal("policy_invalid", d.Code));
        Assert.Equal(["maxEntries", "forbid.parts[0]"], error.Diagnostics.Select(d => d.Args.GetProperty("key").GetString()));
        Assert.StartsWith("the policy is not valid: ", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void A_policy_that_cannot_be_written_as_JSON_throws_ArgumentException()
    {
        Assert.Throws<ArgumentException>(() => new Verifier(new JsonObject { ["maxEntries"] = double.NaN }, Repo.EngineLibrary));
    }

    [Fact]
    public void A_policy_nested_too_deep_for_JSON_throws_ArgumentException()
    {
        var policy = new JsonObject();
        var inner = policy;
        for (var depth = 0; depth < 2000; depth++)
        {
            var next = new JsonObject();
            inner["a"] = next;
            inner = next;
        }

        Assert.Throws<ArgumentException>(() => new Verifier(policy, Repo.EngineLibrary));
    }

    [Fact]
    public void A_policy_holding_a_value_of_a_type_JSON_cannot_write_throws_ArgumentException()
    {
        var policy = new JsonObject { ["maxEntries"] = JsonValue.Create(typeof(int)) };

        Assert.Throws<ArgumentException>(() => new Verifier(policy, Repo.EngineLibrary));
    }

    [Fact]
    public void A_missing_engine_library_throws_LibraryNotFoundException()
    {
        Assert.Throws<LibraryNotFoundException>(() => new Verifier(null, "/elsewhere.so", _ => null));
    }

    [Fact]
    public void A_panic_of_the_engine_is_a_failure_with_the_panic_error_and_no_diagnostics()
    {
        var verifier = new Verifier(null, new FakeCall { PathWith = (_, _) => (StatusDiagnostic.Panic, null), BytesWith = (_, _) => (StatusDiagnostic.Panic, null) });

        var byPath = verifier.Verify(Table);
        var byBytes = verifier.VerifyBytes([]);

        Assert.Equal("panic", byPath.Error?.Code);
        Assert.Equal("panic", byBytes.Error?.Code);
        Assert.Empty(byPath.Diagnostics);
    }

    [Fact]
    public void A_panic_checking_the_policy_throws_PolicyInvalidException_with_the_panic_error()
    {
        var call = new FakeCall { PolicyWith = _ => (StatusDiagnostic.Panic, null) };

        var error = Assert.Throws<PolicyInvalidException>(() => new Verifier(new JsonObject(), call));

        Assert.Equal(["panic"], error.Diagnostics.Select(d => d.Code));
    }

    [Fact]
    public void An_invalid_arg_checking_the_policy_throws_XlsxGraftException()
    {
        var call = new FakeCall { PolicyWith = _ => (StatusDiagnostic.InvalidArg, null) };

        Assert.Throws<XlsxGraftException>(() => new Verifier(new JsonObject(), call));
    }

    [Fact]
    public void The_engines_io_error_for_the_policy_check_comes_first_in_the_exceptions_diagnostics()
    {
        var json = Encoding.UTF8.GetBytes("{\"ok\":false,\"error\":{\"code\":\"io\",\"message\":\"gone\"},\"diagnostics\":[]}");
        var call = new FakeCall { PolicyWith = _ => (StatusDiagnostic.Ok, json) };

        var error = Assert.Throws<PolicyInvalidException>(() => new Verifier(new JsonObject(), call));

        Assert.Equal("io", Assert.Single(error.Diagnostics).Code);
        Assert.Equal("the policy is not valid: gone", error.Message);
    }

    private sealed class FakeCall : VerifyCall
    {
        internal FakeCall()
            : base(Library.Open(Repo.EngineLibrary, _ => null))
        {
        }

        internal Func<byte[], byte[], (int, byte[]?)>? PathWith { get; set; }

        internal Func<byte[], byte[], (int, byte[]?)>? BytesWith { get; set; }

        internal Func<byte[], (int, byte[]?)>? PolicyWith { get; set; }

        internal override (int Status, byte[]? Json) Path(byte[] path, byte[] options) =>
            PathWith?.Invoke(path, options) ?? base.Path(path, options);

        internal override (int Status, byte[]? Json) Bytes(ReadOnlySpan<byte> data, byte[] options) =>
            BytesWith?.Invoke(data.ToArray(), options) ?? base.Bytes(data, options);

        internal override (int Status, byte[]? Json) Policy(byte[] policy) =>
            PolicyWith?.Invoke(policy) ?? base.Policy(policy);
    }
}
