// The package downloads nothing (docs/sdk.md § Finding the engine library):
// no class of it refers to a networking or process type. That JNA is its one
// dependency is checked on the published POM, by script/check-installed-jar.

package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.net.URISyntaxException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;

class NoDownloadTest {

  // A class that used a URL, a socket or a process names its type in its
  // constant pool, even where no signature does (java/lang/Process covers
  // ProcessBuilder too).
  private static final List<String> FORBIDDEN =
      Arrays.asList("java/net/", "javax/net/", "java/lang/Process");

  @Test
  void noClassRefersToANetworkingOrProcessType() throws IOException, URISyntaxException {
    Path classes =
        Paths.get(Engine.class.getProtectionDomain().getCodeSource().getLocation().toURI());
    List<Path> files;
    try (Stream<Path> walk = Files.walk(classes)) {
      files = walk.filter(p -> p.toString().endsWith(".class")).collect(Collectors.toList());
    }

    // A sweep over no classes reports no offenders and proves nothing.
    assertTrue(files.stream().anyMatch(p -> p.endsWith("io/github/kengos/xlsxgraft/Engine.class")));
    for (Path file : files) {
      String constants = new String(Files.readAllBytes(file), StandardCharsets.ISO_8859_1);
      for (String type : FORBIDDEN) {
        assertFalse(constants.contains(type), file + " refers to " + type);
      }
    }
  }
}
