package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.function.Function;
import org.junit.jupiter.api.Test;

class PathInputTest {

  private static final Path TABLE = Repo.ROOT.resolve("fixtures/table.xlsx");

  private static final Function<byte[], Result<Void>> SUCCEEDS = data -> Result.ok(null);

  @Test
  void readVerifiesTheBytesOfAFileAndFailsWithAnIoErrorForOneThatCannotBeRead() {
    Result<Void> read = PathInput.read(TABLE, SUCCEEDS);
    Result<Void> missing = PathInput.read(Paths.get("/nowhere/\u00e9.xlsx"), SUCCEEDS);

    assertTrue(read.success());
    assertFalse(missing.success());
    assertTrue(missing.diagnostics().isEmpty());
    assertEquals("io", missing.error().code());
    assertTrue(
        missing.error().message().startsWith("the file could not be read: "),
        missing.error().message());
  }

  @Test
  void anIoFailureSaysItsMessageOrElseItsClassName() {
    assertEquals("gone", PathInput.reason(new IOException("gone")));
    assertEquals("java.io.IOException", PathInput.reason(new IOException()));
  }
}
