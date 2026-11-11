package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class VerifyReportTest {

  private static final String RECORD =
      "{\"code\":\"part_denied\",\"severity\":\"error\",\"message\":\"m\",\"args\":{},"
          + "\"location\":{},\"decidedBy\":null,\"fix\":null,\"origin\":\"x\"}";

  @Test
  void aResultWhoseOkIsTrueIsASuccessWithNoValueWhateverDiagnosticsItHas() {
    Result<Void> empty = VerifyReport.call(0, bytes(report("true", "null", "")));
    Result<Void> noted = VerifyReport.call(0, bytes(report("true", "null", RECORD)));

    assertTrue(empty.success());
    assertNull(empty.value());
    assertNull(empty.error());
    assertTrue(empty.diagnostics().isEmpty());
    assertTrue(noted.success());
    assertEquals(1, noted.diagnostics().size());
  }

  @Test
  void aResultWhoseOkIsFalseFailsWithTheEnginesDiagnosticsAndTheIoErrorAsTheResultsError() {
    Result<Void> refused = VerifyReport.call(0, bytes(report("false", "null", RECORD)));
    Result<Void> io =
        VerifyReport.call(0, bytes(report("false", "{\"code\":\"io\",\"message\":\"gone\"}", "")));

    assertFalse(refused.success());
    assertNull(refused.error());
    assertEquals(Arrays.asList("part_denied"), Arrays.asList(refused.diagnostics().get(0).code()));
    assertFalse(io.success());
    assertEquals("io", io.error().code());
    assertEquals("gone", io.error().message());
    assertEquals("error", io.error().severity());
    assertTrue(io.diagnostics().isEmpty());
  }

  @Test
  void aStatusOtherThanOkAndInvalidArgFailsWithAnErrorNamingTheStatus() {
    assertEquals("panic", VerifyReport.call(2, null).error().code());
    assertEquals("null_arg", VerifyReport.call(1, null).error().code());
    assertEquals("unknown_status", VerifyReport.call(99, null).error().code());
    assertTrue(VerifyReport.call(2, null).diagnostics().isEmpty());
    assertFalse(VerifyReport.call(2, null).success());
  }

  @Test
  void invalidArgThrowsBecauseOnlyACallThePackageBuiltWronglyGetsIt() {
    XlsxGraftException error =
        assertThrows(XlsxGraftException.class, () -> VerifyReport.call(4, null));

    assertEquals(
        "the engine refused a call this package built (status 4): a bug of the package",
        error.getMessage());
  }

  @ParameterizedTest
  @ValueSource(
      strings = {
        "not json",
        "{}",
        "[]",
        "{\"ok\":1,\"error\":null,\"diagnostics\":[]}",
        "{\"ok\":true,\"error\":null,\"diagnostics\":[1]}",
        "{\"ok\":true,\"error\":null}",
        "{\"error\":null,\"diagnostics\":[]}",
        "{\"ok\":false,\"diagnostics\":[]}",
        "{\"ok\":false,\"error\":3,\"diagnostics\":[]}",
        "{\"ok\":false,\"error\":{\"code\":\"io\"},\"diagnostics\":[]}",
      })
  void jsonThatIsNotAResultFailsWithDiagnosticsUnreadable(String json) {
    Result<Void> result = VerifyReport.call(0, bytes(json));

    assertFalse(result.success());
    assertTrue(result.diagnostics().isEmpty());
    assertEquals("diagnostics_unreadable", result.error().code());
    assertTrue(
        result.error().message().startsWith("the engine's result could not be read: "),
        result.error().message());
  }

  @Test
  void aResultWhoseOkIsTrueIsASuccessEvenWhenItsErrorIsBroken() {
    Result<Void> result =
        VerifyReport.call(0, bytes("{\"ok\":true,\"error\":3,\"diagnostics\":[]}"));
    Result<Void> missing = VerifyReport.call(0, bytes("{\"ok\":true,\"diagnostics\":[]}"));

    assertTrue(result.success());
    assertNull(result.error());
    assertTrue(missing.success());
  }

  @Test
  void bytesThatAreNotUtf8FailWithDiagnosticsUnreadable() {
    Result<Void> result = VerifyReport.call(0, new byte[] {(byte) 0xff});

    assertFalse(result.success());
    assertEquals("diagnostics_unreadable", result.error().code());
    assertTrue(result.diagnostics().isEmpty());
  }

  private static String report(String ok, String error, String records) {
    return "{\"ok\":"
        + ok
        + ",\"failOn\":\"error\",\"counts\":{},\"error\":"
        + error
        + ",\"diagnostics\":["
        + records
        + "]}";
  }

  private static byte[] bytes(String json) {
    return json.getBytes(StandardCharsets.UTF_8);
  }
}
