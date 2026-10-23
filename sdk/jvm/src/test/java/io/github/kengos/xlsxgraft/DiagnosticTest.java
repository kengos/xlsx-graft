package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;

import org.junit.jupiter.api.Test;

class DiagnosticTest {

  private final Diagnostic panic = new Diagnostic("panic", "it panicked");

  @Test
  void isEqualToADiagnosticOfTheSameParts() {
    assertEquals(new Diagnostic("panic", "it panicked"), panic);
    assertEquals(new Diagnostic("panic", "it panicked").hashCode(), panic.hashCode());
    assertNotEquals(new Diagnostic("panic", "something else"), panic);
    assertNotEquals(new Diagnostic("null_arg", "it panicked"), panic);
    // assertNotEquals would ask the String, not the Diagnostic.
    assertFalse(panic.equals("panic: it panicked"));
  }

  @Test
  void printsAsCodeThenMessage() {
    assertEquals("panic: it panicked", panic.toString());
  }
}
