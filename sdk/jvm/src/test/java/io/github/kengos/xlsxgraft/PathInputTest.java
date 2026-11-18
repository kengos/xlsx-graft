package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
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
  void aPathThatIsUnicodeIsGivenToByPathAsAnAbsolutePathEncodedAsUtf8() {
    Result<byte[]> result =
        PathInput.call(
            Paths.get("fixtures/\u00e9.xlsx"),
            encoded -> Result.ok(encoded),
            data -> Result.<byte[]>ok(new byte[0]));

    String absolute = Paths.get("fixtures/\u00e9.xlsx").toAbsolutePath().toString();
    assertEquals(absolute, new String(result.value(), StandardCharsets.UTF_8));
  }

  @Test
  void aPathThatIsNotUnicodeIsReadAndItsBytesGivenToByBytes() throws IOException {
    Result<byte[]> result =
        PathInput.call(
            NonUnicodePath.standingFor(TABLE),
            encoded -> Result.<byte[]>failure(StatusDiagnostic.error("wrong", "by path")),
            data -> Result.ok(data));
    Result<byte[]> missing =
        PathInput.call(
            NonUnicodePath.standingFor(Paths.get("/nowhere/book.xlsx")),
            encoded -> Result.<byte[]>failure(StatusDiagnostic.error("wrong", "by path")),
            data -> Result.ok(data));

    assertTrue(result.success());
    assertArrayEquals(Files.readAllBytes(TABLE), result.value());
    assertEquals("io", missing.error().code());
    assertNull(missing.value());
  }

  @Test
  void anIoFailureSaysItsMessageOrElseItsClassName() {
    assertEquals("gone", PathInput.reason(new IOException("gone")));
    assertEquals("java.io.IOException", PathInput.reason(new IOException()));
  }
}
