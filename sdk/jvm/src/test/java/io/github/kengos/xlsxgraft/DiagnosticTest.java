package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Collections;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class DiagnosticTest {

  private static final Map<String, Object> NONE = Collections.emptyMap();

  private final Diagnostic panic = new Diagnostic("panic", "error", "it panicked");

  @Test
  void isEqualToADiagnosticOfTheSameParts() {
    assertEquals(new Diagnostic("panic", "error", "it panicked"), panic);
    assertEquals(new Diagnostic("panic", "error", "it panicked").hashCode(), panic.hashCode());
    assertNotEquals(new Diagnostic("panic", "error", "something else"), panic);
    assertNotEquals(new Diagnostic("null_arg", "error", "it panicked"), panic);
    assertNotEquals(new Diagnostic("panic", "warning", "it panicked"), panic);
    // assertNotEquals would ask the String, not the Diagnostic.
    assertFalse(panic.equals("panic: it panicked"));
  }

  @Test
  void isEqualByItsJsonFieldsToo() {
    Map<String, Object> one = Collections.<String, Object>singletonMap("a", 1L);

    assertNotEquals(with(one, NONE, null, null), panic);
    assertNotEquals(with(NONE, one, null, null), panic);
    assertNotEquals(with(NONE, NONE, one, null), panic);
    assertNotEquals(with(NONE, NONE, null, "do"), panic);
    assertEquals(with(NONE, NONE, one, "do"), with(NONE, NONE, one, "do"));
  }

  @Test
  void oneThePackageMakesHasNoArgsLocationRuleOrFix() {
    assertEquals("error", panic.severity());
    assertTrue(panic.args().isEmpty());
    assertTrue(panic.location().isEmpty());
    assertNull(panic.decidedBy());
    assertNull(panic.fix());
  }

  @Test
  void theEnginesCarryEveryFieldOfTheJson() {
    Book book = XlsxReader.openBytes(Workbook.xlsx(Workbook.unrelated("Lost")));
    book.close();
    List<Diagnostic> diagnostics = book.result().diagnostics();

    Book again = XlsxReader.openBytes(Workbook.xlsx(Workbook.unrelated("Lost")));
    again.close();

    assertFalse(diagnostics.isEmpty());
    assertEquals(diagnostics, again.result().diagnostics());
    Diagnostic first = diagnostics.get(0);
    assertEquals("sheet_relationship_missing", first.code());
    assertEquals(Collections.singletonMap("relationshipId", "rId1"), first.args());
    assertFalse(first.location().isEmpty(), first.location().toString());
    assertEquals("standard", first.decidedBy().get("kind"));
    assertEquals("workbook", ((Map<?, ?>) first.fix()).get("in"));
  }

  @Test
  void printsAsCodeThenMessage() {
    assertEquals("panic: it panicked", panic.toString());
  }

  private static Diagnostic with(
      Map<String, Object> args,
      Map<String, Object> location,
      Map<String, Object> rule,
      Object fix) {
    return new Diagnostic("panic", "error", "it panicked", args, location, rule, fix);
  }
}
