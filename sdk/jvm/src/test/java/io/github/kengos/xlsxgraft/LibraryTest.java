package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.Collections;
import java.util.Map;
import java.util.jar.JarEntry;
import java.util.jar.JarOutputStream;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class LibraryTest {

  private static final Map<String, String> NO_ENV = Collections.emptyMap();

  // Where a test puts a packaged copy; empty unless it does.
  @TempDir Path classpath;

  @Test
  void takesXlsxGraftLibraryOverThePathArgument() throws IOException {
    Library opened = Library.open("/nowhere.so", env(Repo.ENGINE_LIBRARY), loader());

    assertEquals(Repo.ENGINE_LIBRARY, opened.path());
  }

  @Test
  void takesThePathArgumentWhenXlsxGraftLibraryIsEmpty() throws IOException {
    Library opened = Library.open(Repo.ENGINE_LIBRARY, env(""), loader());

    assertEquals(Repo.ENGINE_LIBRARY, opened.path());
  }

  @Test
  void fallsBackToThePackagedCopyOnTheClasspath() throws IOException {
    Path packaged = classpath.resolve("native/libxlsx_graft_capi.so");
    Files.createDirectories(packaged.getParent());
    Files.copy(Paths.get(Repo.ENGINE_LIBRARY), packaged);

    Library opened = Library.open("", NO_ENV, loader());

    assertEquals(packaged.toString(), opened.path());
    assertEquals(Repo.ENGINE_VERSION, VersionCall.call(opened.api(), opened.path()).value());
  }

  @Test
  void extractsThePackagedCopyOutOfAJarOnceForEachClassLoader() throws IOException {
    Path jar = classpath.resolve("with-native.jar");
    try (JarOutputStream out = new JarOutputStream(Files.newOutputStream(jar))) {
      out.putNextEntry(new JarEntry("native/libxlsx_graft_capi.so"));
      Files.copy(Paths.get(Repo.ENGINE_LIBRARY), out);
    }
    URLClassLoader loader = new URLClassLoader(new URL[] {jar.toUri().toURL()}, null);

    Library first = Library.open(null, NO_ENV, loader);
    Library second = Library.open(null, NO_ENV, loader);

    // Inside a jar, native/ has no path of its own: JNA extracts a copy.
    assertFalse(first.path().startsWith(classpath.toString()), first.path());
    assertEquals(first.path(), second.path());
    assertEquals(Repo.ENGINE_VERSION, VersionCall.call(second.api(), second.path()).value());

    URLClassLoader another = new URLClassLoader(new URL[] {jar.toUri().toURL()}, null);
    assertNotEquals(first.path(), Library.open(null, NO_ENV, another).path());
  }

  @Test
  void throwsWithTheWaysToInstallWhenNothingIsFound() {
    // The test classpath carries no native/, so the default loader finds none.
    LibraryNotFoundException error =
        assertThrows(LibraryNotFoundException.class, () -> Library.open(null, NO_ENV));

    String message = error.getMessage();
    assertTrue(message.startsWith("no engine library was found."), message);
    assertTrue(message.contains("set XLSX_GRAFT_LIBRARY"), message);
    assertTrue(message.contains("Engine.version(\"/path/to/libxlsx_graft_capi.so\")"), message);
    assertTrue(message.contains("native/ on the classpath"), message);
  }

  @Test
  void throwsNamingTheFileWhenItIsNotALibrary() throws IOException {
    Path notALibrary = classpath.resolve("not_a_library.so");
    Files.write(notALibrary, "text".getBytes(StandardCharsets.UTF_8));

    LibraryNotFoundException error =
        assertThrows(
            LibraryNotFoundException.class,
            () -> Library.open(notALibrary.toString(), NO_ENV, loader()));

    assertTrue(error.getMessage().startsWith(notALibrary + " could not be loaded"));
    assertInstanceOf(UnsatisfiedLinkError.class, error.getCause());
  }

  @Test
  void looksForEachPlatformsFileNameAndWindowsHasNoLibPrefix() {
    assertEquals(
        Arrays.asList("libxlsx_graft_capi.so", "libxlsx_graft_capi.dylib", "xlsx_graft_capi.dll"),
        Library.NAMES);
  }

  // A class loader over the temporary directory alone, as the classpath of an
  // application that may ship native/.
  private URLClassLoader loader() throws IOException {
    return new URLClassLoader(new URL[] {classpath.toUri().toURL()}, null);
  }

  private static Map<String, String> env(String value) {
    return Collections.singletonMap("XLSX_GRAFT_LIBRARY", value);
  }
}
