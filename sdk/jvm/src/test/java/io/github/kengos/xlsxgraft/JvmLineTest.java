package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class JvmLineTest {

  // The tests run on the JVM of the line being gated, not on the JDK that
  // runs Gradle; Java 8 names itself 1.8.
  @Test
  void runsOnTheJvmOfTheGatedLine() {
    String line = Repo.required("XLSX_GRAFT_JVM_LINE");
    String spec = System.getProperty("java.specification.version");
    System.out.println("tests run on Java " + spec + " (" + System.getProperty("java.home") + ")");

    assertEquals("8".equals(line) ? "1.8" : line, spec);
  }
}
