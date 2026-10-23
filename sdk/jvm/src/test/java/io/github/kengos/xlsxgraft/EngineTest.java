package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.util.Collections;
import java.util.Map;
import org.junit.jupiter.api.Test;

class EngineTest {

  private static final Map<String, String> NO_ENV = Collections.emptyMap();

  @Test
  void versionReturnsTheEnginesVersionFromTheLibrary() {
    Result<String> result = Engine.version();

    assertTrue(result.success());
    assertFalse(result.failed());
    assertEquals(Repo.ENGINE_VERSION, result.value());
    assertTrue(result.diagnostics().isEmpty());
  }

  @Test
  void thePackageVersionTracksTheEngines() throws IOException {
    assertEquals(Repo.ENGINE_VERSION, Repo.packageVersion());
  }

  @Test
  void versionOpensTheLibraryArgumentWhenXlsxGraftLibraryIsUnset() {
    assertEquals(Repo.ENGINE_VERSION, Engine.version(Repo.ENGINE_LIBRARY, NO_ENV).value());

    LibraryNotFoundException error =
        assertThrows(
            LibraryNotFoundException.class,
            () -> Engine.version("/elsewhere/libxlsx_graft_capi.so", NO_ENV));
    assertTrue(
        error.getMessage().startsWith("/elsewhere/libxlsx_graft_capi.so could not be loaded"),
        error.getMessage());
  }

  @Test
  void versionTakesXlsxGraftLibraryOverTheArgument() {
    assertEquals(Repo.ENGINE_VERSION, Engine.version("/nowhere.so").value());
  }
}
