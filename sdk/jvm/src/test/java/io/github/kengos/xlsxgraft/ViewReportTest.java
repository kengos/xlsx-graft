package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.NullAndEmptySource;
import org.junit.jupiter.params.provider.ValueSource;

class ViewReportTest {

  private static final String RECORD =
      "{\"code\":\"part_denied\",\"severity\":\"error\",\"message\":\"m\",\"args\":{},"
          + "\"location\":{},\"decidedBy\":null,\"fix\":null}";

  // Reads the key "shown" of the view, which is all these tests need of a content.
  private static final ViewReport.Content<String> SHOWN =
      view -> EngineDiagnostics.text(view, "shown");

  @Test
  void aViewWhoseOkIsTrueIsASuccessWithItsValueAndDiagnosticsAndNoError() {
    Result<String> result = call(0, view("true", "null", RECORD, "\"A1\""));

    assertTrue(result.success());
    assertEquals("A1", result.value());
    assertNull(result.error());
    assertEquals(1, result.diagnostics().size());
    assertEquals("part_denied", result.diagnostics().get(0).code());
  }

  @Test
  void aViewWhoseOkIsFalseFailsWithItsValueItsDiagnosticsAndItsErrorAsTheResultsError() {
    Result<String> refused = call(0, view("false", "null", RECORD, "\"A1\""));
    Result<String> io =
        call(0, view("false", "{\"code\":\"io\",\"message\":\"gone\"}", "", "\"B2\""));

    assertFalse(refused.success());
    assertEquals("A1", refused.value());
    assertNull(refused.error());
    assertEquals(1, refused.diagnostics().size());
    assertFalse(io.success());
    assertEquals("B2", io.value());
    assertEquals("io", io.error().code());
    assertEquals("gone", io.error().message());
    assertEquals("error", io.error().severity());
    assertTrue(io.diagnostics().isEmpty());
  }

  @Test
  void aKeyThePackageDoesNotKnowIsSkipped() {
    Result<String> result =
        call(
            0,
            "{\"ok\":true,\"error\":null,\"diagnostics\":[],\"shown\":\"A1\",\"newer\":{\"x\":[1]}}");

    assertTrue(result.success());
    assertEquals("A1", result.value());
  }

  @Test
  void aViewWhoseOkIsTrueIsASuccessEvenWhenItsErrorIsBroken() {
    Result<String> broken =
        call(0, "{\"ok\":true,\"error\":3,\"diagnostics\":[],\"shown\":\"A1\"}");
    Result<String> missing = call(0, "{\"ok\":true,\"diagnostics\":[],\"shown\":\"A1\"}");

    assertTrue(broken.success());
    assertNull(broken.error());
    assertTrue(missing.success());
  }

  @Test
  void aStatusOtherThanOkRequestInvalidAndInvalidArgFailsWithAnErrorNamingTheStatusAndNoValue() {
    Result<String> panic = ViewReport.call(new ViewCall.Outcome(2, null, null), SHOWN);
    Result<String> nullArg = ViewReport.call(new ViewCall.Outcome(1, null, null), SHOWN);
    Result<String> unknown = ViewReport.call(new ViewCall.Outcome(99, null, null), SHOWN);

    assertEquals("panic", panic.error().code());
    assertEquals("null_arg", nullArg.error().code());
    assertEquals("unknown_status", unknown.error().code());
    assertFalse(panic.success());
    assertNull(panic.value());
    assertTrue(panic.diagnostics().isEmpty());
  }

  @Test
  void requestInvalidThrowsIllegalArgumentWithTheEnginesMessage() {
    IllegalArgumentException error =
        assertThrows(
            IllegalArgumentException.class,
            () -> ViewReport.call(new ViewCall.Outcome(11, null, "maxRows is 0"), SHOWN));

    assertEquals("maxRows is 0", error.getMessage());
  }

  @ParameterizedTest
  @NullAndEmptySource
  void requestInvalidWithNoMessageThrowsIllegalArgumentWithAFixedMessage(String message) {
    IllegalArgumentException error =
        assertThrows(
            IllegalArgumentException.class,
            () -> ViewReport.call(new ViewCall.Outcome(11, null, message), SHOWN));

    assertEquals(
        "the engine refused a value of the options (status 11) and gave no reason",
        error.getMessage());
  }

  @Test
  void invalidArgThrowsBecauseOnlyACallThePackageBuiltWronglyGetsIt() {
    XlsxGraftException error =
        assertThrows(
            XlsxGraftException.class,
            () -> ViewReport.call(new ViewCall.Outcome(4, null, null), SHOWN));

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
        "{\"ok\":true,\"error\":null,\"diagnostics\":[]}",
        "{\"ok\":true,\"error\":null,\"shown\":\"A1\"}",
        "{\"error\":null,\"diagnostics\":[],\"shown\":\"A1\"}",
        "{\"ok\":1,\"error\":null,\"diagnostics\":[],\"shown\":\"A1\"}",
        "{\"ok\":true,\"error\":null,\"diagnostics\":[1],\"shown\":\"A1\"}",
        "{\"ok\":true,\"error\":null,\"diagnostics\":[],\"shown\":3}",
        "{\"ok\":false,\"diagnostics\":[],\"shown\":\"A1\"}",
        "{\"ok\":false,\"error\":3,\"diagnostics\":[],\"shown\":\"A1\"}",
        "{\"ok\":false,\"error\":{\"code\":\"io\"},\"diagnostics\":[],\"shown\":\"A1\"}",
      })
  void jsonThatIsNotAViewFailsWithDiagnosticsUnreadableAndNoValue(String json) {
    Result<String> result = call(0, json);

    assertFalse(result.success());
    assertNull(result.value());
    assertTrue(result.diagnostics().isEmpty());
    assertEquals("diagnostics_unreadable", result.error().code());
    assertTrue(
        result.error().message().startsWith("the engine's result could not be read: "),
        result.error().message());
  }

  @Test
  void bytesThatAreNotUtf8FailWithDiagnosticsUnreadable() {
    Result<String> result =
        ViewReport.call(new ViewCall.Outcome(0, new byte[] {(byte) 0xff}, null), SHOWN);

    assertEquals("diagnostics_unreadable", result.error().code());
    assertNull(result.value());
  }

  private static Result<String> call(int status, String json) {
    return ViewReport.call(
        new ViewCall.Outcome(status, json.getBytes(StandardCharsets.UTF_8), null), SHOWN);
  }

  private static String view(String ok, String error, String records, String shown) {
    return "{\"ok\":"
        + ok
        + ",\"error\":"
        + error
        + ",\"diagnostics\":["
        + records
        + "],\"shown\":"
        + shown
        + "}";
  }
}
