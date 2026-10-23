// Run by `make sdk:verify:jvm` with the jar the gate published resolved from
// the Maven repository it went to: checks the class it loaded is that jar's,
// what the jar and its POM hold, and that it reaches the engine.
//
//     gradle run --args="<maven repository> <sdk/jvm/gradle.lockfile> <engine/Cargo.toml>"

import io.github.kengos.xlsxgraft.Engine;
import io.github.kengos.xlsxgraft.Result;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.jar.JarFile;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import javax.xml.parsers.DocumentBuilderFactory;
import org.w3c.dom.Element;
import org.w3c.dom.NodeList;

public final class CheckInstalledJar {

  private CheckInstalledJar() {}

  public static void main(String[] args) throws Exception {
    Path repo = Paths.get(args[0]);
    String jna = lockedJna(Paths.get(args[1]));
    String engine = engineVersion(Paths.get(args[2]));

    // The package's version tracks the engine's, so the engine's names the
    // directory it was published to.
    Path published = repo.resolve("io/github/kengos/xlsx-graft/" + engine);
    Path jar = published.resolve("xlsx-graft-" + engine + ".jar");
    Path loaded =
        Paths.get(Engine.class.getProtectionDomain().getCodeSource().getLocation().toURI());

    check(Files.exists(jar), published + " holds no xlsx-graft-" + engine + ".jar");
    check(
        sha256(loaded).equals(sha256(jar)),
        "loaded Engine from " + loaded + ", which is not " + jar);
    for (String classifier : new String[] {"sources", "javadoc"}) {
      Path attached = published.resolve("xlsx-graft-" + engine + "-" + classifier + ".jar");
      check(Files.exists(attached), "the package publishes no " + attached.getFileName());
    }

    try (JarFile file = new JarFile(jar.toFile())) {
      check(
          Collections.list(file.entries()).stream()
              .noneMatch(e -> e.getName().startsWith("native/")),
          "the jar ships native/");
      String module = file.getManifest().getMainAttributes().getValue("Automatic-Module-Name");
      check(
          "io.github.kengos.xlsxgraft".equals(module),
          "the jar's Automatic-Module-Name is " + module);
    }

    List<String> dependencies = dependencies(published.resolve("xlsx-graft-" + engine + ".pom"));
    String expected = "net.java.dev.jna:jna:" + jna + ":runtime";
    check(
        dependencies.equals(Collections.singletonList(expected)),
        "xlsx-graft depends on " + dependencies + ", not on " + expected + " alone");

    // As JvmLineTest does for the tests: the jar is loaded on the line's JVM,
    // not on the JDK that runs Gradle.
    String line = System.getenv("XLSX_GRAFT_JVM_LINE");
    String spec = System.getProperty("java.specification.version");
    check(
        spec.equals("8".equals(line) ? "1.8" : line),
        "the jar was loaded on Java " + spec + ", not on the gated line " + line);

    Result<String> result = Engine.version();
    check(result.success(), "Engine.version() failed: " + result.diagnostics());
    check(
        engine.equals(result.value()),
        "Engine.version() is " + result.value() + ", the engine is " + engine);

    System.out.println(
        "installed io.github.kengos:xlsx-graft:"
            + engine
            + " from "
            + repo
            + " loads engine "
            + result.value()
            + " on Java "
            + spec);
  }

  // The JNA version the SDK's lockfile pins, from its
  // `net.java.dev.jna:jna:<version>=<configurations>` line.
  private static String lockedJna(Path lockfile) throws IOException {
    for (String line : Files.readAllLines(lockfile, StandardCharsets.UTF_8)) {
      if (line.startsWith("net.java.dev.jna:jna:")) {
        return line.substring("net.java.dev.jna:jna:".length(), line.indexOf('='));
      }
    }
    throw new IllegalStateException(lockfile + " pins no net.java.dev.jna:jna");
  }

  private static String engineVersion(Path cargo) throws IOException {
    Matcher m =
        Pattern.compile(
                "^\\[workspace\\.package\\]\\s*\\nversion = \"([^\"]+)\"", Pattern.MULTILINE)
            .matcher(new String(Files.readAllBytes(cargo), StandardCharsets.UTF_8));
    check(m.find(), cargo + " has no [workspace.package] version");
    return m.group(1);
  }

  // Each <dependency> of the POM as group:artifact:version:scope.
  private static List<String> dependencies(Path pom) throws Exception {
    NodeList nodes =
        DocumentBuilderFactory.newInstance()
            .newDocumentBuilder()
            .parse(pom.toFile())
            .getElementsByTagName("dependency");
    List<String> found = new ArrayList<>();
    for (int i = 0; i < nodes.getLength(); i++) {
      Element dependency = (Element) nodes.item(i);
      found.add(
          text(dependency, "groupId")
              + ":"
              + text(dependency, "artifactId")
              + ":"
              + text(dependency, "version")
              + ":"
              + text(dependency, "scope"));
    }
    return found;
  }

  private static String text(Element parent, String tag) {
    NodeList nodes = parent.getElementsByTagName(tag);
    return nodes.getLength() == 0 ? "" : nodes.item(0).getTextContent();
  }

  private static String sha256(Path path) throws IOException, NoSuchAlgorithmException {
    StringBuilder hex = new StringBuilder();
    for (byte b : MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(path))) {
      hex.append(String.format("%02x", b));
    }
    return hex.toString();
  }

  private static void check(boolean condition, String failure) {
    if (!condition) {
      System.err.println(failure);
      System.exit(1);
    }
  }
}
