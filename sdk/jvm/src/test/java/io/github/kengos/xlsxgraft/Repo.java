// What every test shares: the repository, the engine library and the engine's
// version.

package io.github.kengos.xlsxgraft;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Properties;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

final class Repo {

  /** The repository's root: Gradle runs the tests in sdk/jvm. */
  static final Path ROOT = Paths.get("").toAbsolutePath().resolve("../..").normalize();

  /** The library `make engine:capi-lib` built; the gate points XLSX_GRAFT_LIBRARY at it. */
  static final String ENGINE_LIBRARY = required("XLSX_GRAFT_LIBRARY");

  /** The engine workspace's version, from engine/Cargo.toml's [workspace.package]. */
  static final String ENGINE_VERSION = engineVersion();

  private Repo() {}

  /** An environment variable the gate's image sets. */
  static String required(String name) {
    String value = System.getenv(name);
    if (value == null) {
      throw new IllegalStateException(name + " is not set; the gate sets it");
    }
    return value;
  }

  /** The package's version, from sdk/jvm/gradle.properties. */
  static String packageVersion() throws IOException {
    Properties properties = new Properties();
    try (InputStream in = Files.newInputStream(ROOT.resolve("sdk/jvm/gradle.properties"))) {
      properties.load(in);
    }
    return properties.getProperty("version");
  }

  private static String engineVersion() {
    try {
      String cargo =
          new String(Files.readAllBytes(ROOT.resolve("engine/Cargo.toml")), StandardCharsets.UTF_8);
      Matcher m =
          Pattern.compile(
                  "^\\[workspace\\.package\\]\\s*\\nversion = \"([^\"]+)\"", Pattern.MULTILINE)
              .matcher(cargo);
      if (!m.find()) {
        throw new IllegalStateException("engine/Cargo.toml has no [workspace.package] version");
      }
      return m.group(1);
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }
}
