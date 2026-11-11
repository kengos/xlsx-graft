package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import org.junit.jupiter.api.Test;

class VerifierTest {

  private static final Map<String, String> NO_ENV = Collections.emptyMap();
  private static final Path MACRO = Repo.ROOT.resolve("fixtures/macro.xlsm");
  private static final Path TABLE = Repo.ROOT.resolve("fixtures/table.xlsx");
  private static final Path DYNAMIC = Repo.ROOT.resolve("fixtures/dynamic-array.xlsx");

  private final Verifier verifier = verifier(null);

  @Test
  void verifyFailsAWorkbookTheEngineRefusesWithItsDiagnosticsAndNoValue() {
    Result<Void> result = verifier.verify(MACRO);

    assertFalse(result.success());
    assertNull(result.error());
    assertNull(result.value());
    assertEquals(Arrays.asList("part_denied", "part_denied"), codes(result));
    Diagnostic first = result.diagnostics().get(0);
    assertEquals("error", first.severity());
    assertFalse(first.args().isEmpty());
    assertNotNull(first.decidedBy());
    assertEquals("template", ((Map<?, ?>) first.fix()).get("in"));
  }

  @Test
  void verifyLetsThroughAPartThatIsAllowedAndPassesAWorkbookWithNoFault() {
    Result<Void> allowed =
        verifier.verify(MACRO, VerifyRules.none().allowParts(Arrays.asList("macro")));
    Result<Void> table = verifier.verify(TABLE);

    assertTrue(allowed.success());
    assertTrue(allowed.diagnostics().isEmpty());
    assertTrue(table.success());
    assertTrue(table.diagnostics().isEmpty());
  }

  @Test
  void verifyTakesARelativePathFromTheWorkingDirectory() {
    Path relative = Paths.get("").toAbsolutePath().relativize(TABLE);

    assertFalse(relative.isAbsolute());
    assertTrue(verifier.verify(relative).success());
  }

  @Test
  void verifyFailsWithAnIoErrorForAFileThatCannotBeRead() {
    Result<Void> result = verifier.verify(Paths.get("/nowhere/book.xlsx"));

    assertFalse(result.success());
    assertTrue(result.diagnostics().isEmpty());
    assertEquals("io", result.error().code());
    assertTrue(result.error().message().contains("/nowhere/book.xlsx"), result.error().message());
  }

  @Test
  void aFaultOfTheRulesIsDiagnosticsAndTheWorkbookIsNotOpened() {
    VerifyRules rules =
        VerifyRules.none()
            .targets(Arrays.asList("excel 1990"))
            .allowFunctions(Arrays.asList("_xlfn.SEQUENCE"));

    Result<Void> result = verifier.verify(Paths.get("/nowhere/book.xlsx"), rules);

    assertFalse(result.success());
    assertNull(result.error());
    assertEquals(Arrays.asList("target_invalid", "rules_invalid"), inReverseOrder(codes(result)));
    Diagnostic invalid = result.diagnostics().get(codes(result).indexOf("rules_invalid"));
    assertEquals("prefixed", invalid.args().get("reason"));
  }

  @Test
  void failOnWarnFailsAWarningOnlyWhenTheRulesSaySo() {
    VerifyRules bestEffort = VerifyRules.none().bestEffort(Arrays.asList("excel 2016"));

    Result<Void> lenient = verifier.verify(DYNAMIC, bestEffort);
    Result<Void> strict = verifier.verify(DYNAMIC, bestEffort.failOn("warn"));

    assertTrue(lenient.success());
    assertTrue(severities(lenient).contains("warn"));
    assertFalse(strict.success());
    assertNull(strict.error());
    assertFalse(severities(strict).contains("error"));
  }

  @Test
  void readVerifiesTheBytesOfAFileAndFailsWithAnIoErrorForOneThatCannotBeRead() {
    byte[] options = "{\"policy\":null,\"rules\":{}}".getBytes(StandardCharsets.UTF_8);

    Result<Void> read = verifier.read(TABLE, options);
    Result<Void> missing = verifier.read(Paths.get("/nowhere/é.xlsx"), options);

    assertTrue(read.success());
    assertFalse(missing.success());
    assertTrue(missing.diagnostics().isEmpty());
    assertEquals("io", missing.error().code());
    assertTrue(
        missing.error().message().startsWith("the file could not be read: "),
        missing.error().message());
  }

  @Test
  void verifyBytesVerifiesTheBytesAndBytesThatAreNoWorkbookFailWithADiagnosticAndNoError()
      throws IOException {
    Result<Void> table = verifier.verifyBytes(Files.readAllBytes(TABLE));
    Result<Void> empty = verifier.verifyBytes(new byte[0]);
    Result<Void> macro =
        verifier.verifyBytes(
            Files.readAllBytes(MACRO), VerifyRules.none().allowParts(Arrays.asList("macro")));

    assertTrue(table.success());
    assertTrue(macro.success());
    assertFalse(empty.success());
    assertNull(empty.error());
    assertFalse(empty.diagnostics().isEmpty());
  }

  @Test
  void oneVerifierMayBeSharedByThreadsThatCallItAtOnce() throws Exception {
    byte[] bytes = Files.readAllBytes(MACRO);
    ExecutorService pool = Executors.newFixedThreadPool(4);
    try {
      List<Future<List<String>>> futures = new ArrayList<>();
      for (int thread = 0; thread < 4; thread++) {
        futures.add(pool.submit(() -> codes(verifier.verifyBytes(bytes))));
      }
      for (Future<List<String>> future : futures) {
        assertEquals(Arrays.asList("part_denied", "part_denied"), future.get());
      }
    } finally {
      pool.shutdown();
    }
  }

  @Test
  void thePolicyIsGivenToTheEngineSoItsMaxEntriesFailsTheVerification() {
    Result<Void> result = verifier(policy("maxEntries", 1)).verify(TABLE);

    assertFalse(result.success());
    assertEquals("policy", result.diagnostics().get(0).decidedBy().get("source"));
  }

  @Test
  void thePolicysFailOnMakesAWarningFail() {
    Verifier strict = verifier(policy("failOn", "warn"));

    Result<Void> result =
        strict.verify(DYNAMIC, VerifyRules.none().bestEffort(Arrays.asList("excel 2016")));

    assertFalse(result.success());
  }

  @Test
  void theVerifierKeepsThePolicyItCheckedWhenTheCallerChangesTheirs() {
    Map<String, Object> policy = policy("maxEntries", 1);
    Verifier kept = verifier(policy);
    policy.put("maxEntries", 10000);

    assertFalse(kept.verify(TABLE).success());
  }

  @Test
  void aPolicyTheEngineDoesNotTakeThrowsPolicyInvalidWithItsDiagnostics() {
    Map<String, Object> policy = policy("maxEntries", 0);
    policy.put(
        "forbid",
        Collections.singletonMap(
            "parts", Collections.singletonList(Collections.singletonMap("kind", "nope"))));

    PolicyInvalidException error =
        assertThrows(PolicyInvalidException.class, () -> verifier(policy));

    List<String> keys = new ArrayList<>();
    for (Diagnostic diagnostic : error.diagnostics()) {
      assertEquals("policy_invalid", diagnostic.code());
      keys.add((String) diagnostic.args().get("key"));
    }
    assertEquals(Arrays.asList("maxEntries", "forbid.parts[0]"), keys);
    assertTrue(error.getMessage().startsWith("the policy is not valid: "), error.getMessage());
    assertThrows(
        UnsupportedOperationException.class,
        () -> error.diagnostics().add(error.diagnostics().get(0)));
  }

  @Test
  void policyInvalidJoinsTheMessagesOfTheErrorAndTheDiagnostics() {
    Diagnostic error = StatusDiagnostic.error("io", "gone");
    Diagnostic fault = StatusDiagnostic.error("policy_invalid", "bad key");

    PolicyInvalidException exception = new PolicyInvalidException(Arrays.asList(error, fault));

    assertEquals("the policy is not valid: gone; bad key", exception.getMessage());
    assertEquals(Arrays.asList(error, fault), exception.diagnostics());
  }

  @Test
  void thePolicysKeysAreNotChangedSoSnakeCaseIsNotTheEnginesName() {
    PolicyInvalidException error =
        assertThrows(PolicyInvalidException.class, () -> verifier(policy("max_entries", 1)));

    assertEquals("policy_invalid", error.diagnostics().get(0).code());
    assertEquals("max_entries", error.diagnostics().get(0).args().get("key"));
  }

  @Test
  void aPolicyCheckThatFailsAsACallPutsItsErrorFirstInPolicyInvalid() {
    VerifyCallTest.Fake fake = new VerifyCallTest.Fake();
    fake.makeStatus = 2;
    fake.writesHandle = false;

    PolicyInvalidException error =
        assertThrows(
            PolicyInvalidException.class, () -> new Verifier(policy("maxEntries", 1), fake.call()));

    assertEquals(1, error.diagnostics().size());
    assertEquals("panic", error.diagnostics().get(0).code());
    assertEquals(
        "the policy is not valid: the engine panicked; the call did not complete",
        error.getMessage());
  }

  @Test
  void optionsOfExactlyTwoMebibytesCallTheEngineAndOneByteMoreAreRefused() {
    VerifyCallTest.Fake fake = new VerifyCallTest.Fake();
    fake.makeStatus = 2;
    fake.writesHandle = false;
    Verifier fakeVerifier = new Verifier(null, fake.call());
    int overhead = "{\"policy\":null,\"rules\":{\"functions\":{\"allow\":[\"\"]}}}".length();

    Result<Void> exact =
        fakeVerifier.verifyBytes(new byte[0], allowing(repeat("A", (2 << 20) - overhead)));
    IllegalArgumentException over =
        assertThrows(
            IllegalArgumentException.class,
            () ->
                fakeVerifier.verifyBytes(
                    new byte[0], allowing(repeat("A", (2 << 20) - overhead + 1))));

    assertEquals("panic", exact.error().code());
    assertEquals(1, fake.made.size());
    assertTrue(over.getMessage().contains("are 2097153 bytes of JSON"), over.getMessage());
  }

  @Test
  void optionsWithinTwoMebibytesInCharactersButOverInUtf8BytesAreRefused() {
    VerifyRules rules = allowing(repeat("\u00e9", 1 << 20));

    IllegalArgumentException error =
        assertThrows(
            IllegalArgumentException.class, () -> verifier.verifyBytes(new byte[0], rules));

    assertTrue(error.getMessage().endsWith(" bytes of JSON; the engine takes 2097152"));
  }

  @Test
  void aCallTheEngineRefusesAsInvalidArgThrowsAndClosesNothing() {
    VerifyCallTest.Fake fake = new VerifyCallTest.Fake();
    fake.makeStatus = 4;
    fake.writesHandle = false;

    XlsxGraftException error =
        assertThrows(
            XlsxGraftException.class,
            () -> new Verifier(null, fake.call()).verifyBytes(new byte[0]));

    assertTrue(error.getMessage().endsWith("a bug of the package"), error.getMessage());
    assertEquals(0, fake.closed.size());
  }

  @Test
  void aFailureStatusThatCameWithAHandleStillClosesTheHandle() {
    VerifyCallTest.Fake fake = new VerifyCallTest.Fake();
    fake.makeStatus = 2;

    Result<Void> result = new Verifier(null, fake.call()).verifyBytes(new byte[0]);

    assertEquals("panic", result.error().code());
    assertEquals(Collections.singletonList(fake.handle), fake.closed);
  }

  @Test
  void anIoFailureSaysItsMessageOrElseItsClassName() {
    assertEquals("gone", Verifier.reason(new IOException("gone")));
    assertEquals("java.io.IOException", Verifier.reason(new IOException()));
  }

  @Test
  void aPolicyThatJsonCannotWriteThrowsIllegalArgument() {
    assertThrows(IllegalArgumentException.class, () -> verifier(policy("maxEntries", Double.NaN)));
    assertThrows(
        IllegalArgumentException.class, () -> verifier(policy("maxEntries", new Object())));
    assertThrows(IllegalArgumentException.class, () -> verifier(policy("maxEntries", "\ud800")));
  }

  @Test
  void aRuleWithAnUnpairedSurrogateThrowsIllegalArgument() {
    VerifyRules rules = VerifyRules.none().targets(Arrays.asList("\udcff"));

    assertThrows(IllegalArgumentException.class, () -> verifier.verifyBytes(new byte[0], rules));
  }

  @Test
  void optionsOfMoreThanTwoMebibytesAreRefusedBeforeTheEngineIsCalled() {
    VerifyRules rules = VerifyRules.none().allowFunctions(Arrays.asList(repeat("A", 2 << 20)));

    IllegalArgumentException error =
        assertThrows(IllegalArgumentException.class, () -> verifier.verify(TABLE, rules));

    assertTrue(error.getMessage().endsWith(" bytes of JSON; the engine takes 2097152"));
    assertTrue(error.getMessage().startsWith("the policy and the rules are "));
  }

  @Test
  void aNullArgumentThrowsNullPointer() {
    assertThrows(NullPointerException.class, () -> verifier.verify(null));
    assertThrows(NullPointerException.class, () -> verifier.verify(TABLE, null));
    assertThrows(NullPointerException.class, () -> verifier.verifyBytes(null));
    assertThrows(NullPointerException.class, () -> verifier.verifyBytes(new byte[0], null));
  }

  @Test
  void theConstructorsWithFewerArgumentsFindTheLibraryTheEnvironmentNames() {
    // The gate sets XLSX_GRAFT_LIBRARY, which wins over the library argument.
    assertTrue(new Verifier().verify(TABLE).success());
    assertTrue(new Verifier(null).verify(TABLE).success());
    assertTrue(new Verifier(policy("maxEntries", 1), Repo.ENGINE_LIBRARY).verify(TABLE).failed());
  }

  private static Verifier verifier(Map<String, ?> policy) {
    return new Verifier(policy, Repo.ENGINE_LIBRARY, NO_ENV);
  }

  private static Map<String, Object> policy(String key, Object value) {
    Map<String, Object> policy = new LinkedHashMap<>();
    policy.put(key, value);
    return policy;
  }

  private static List<String> codes(Result<Void> result) {
    List<String> codes = new ArrayList<>();
    for (Diagnostic diagnostic : result.diagnostics()) {
      codes.add(diagnostic.code());
    }
    return codes;
  }

  private static List<String> severities(Result<Void> result) {
    List<String> severities = new ArrayList<>();
    for (Diagnostic diagnostic : result.diagnostics()) {
      severities.add(diagnostic.severity());
    }
    return severities;
  }

  private static List<String> inReverseOrder(List<String> values) {
    List<String> copy = new ArrayList<>(values);
    Collections.sort(copy, Collections.reverseOrder());
    return copy;
  }

  private static VerifyRules allowing(String function) {
    return VerifyRules.none().allowFunctions(Arrays.asList(function));
  }

  private static String repeat(String text, int times) {
    StringBuilder out = new StringBuilder();
    for (int count = 0; count < times; count++) {
      out.append(text);
    }
    return out.toString();
  }
}
