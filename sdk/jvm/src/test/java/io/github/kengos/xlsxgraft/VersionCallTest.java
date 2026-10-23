package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.sun.jna.Native;
import com.sun.jna.ptr.PointerByReference;
import java.util.Collections;
import java.util.List;
import org.junit.jupiter.api.Test;

class VersionCallTest {

  @Test
  void aPanicStatusReturnsAFailureCarryingThePanicDiagnostic() {
    Result<String> result = callReturning(2);

    assertFalse(result.success());
    assertTrue(result.failed());
    assertNull(result.value());
    assertEquals(
        Collections.singletonList(
            new Diagnostic("panic", "the engine panicked; the call did not complete")),
        result.diagnostics());
  }

  @Test
  void aNullArgStatusReturnsAFailureCarryingTheNullArgDiagnostic() {
    assertEquals("null_arg", callReturning(1).diagnostics().get(0).code());
  }

  @Test
  void aStatusThisPackageDoesNotKnowReturnsAFailureNamingIt() {
    List<Diagnostic> diagnostics = callReturning(7).diagnostics();

    assertEquals(1, diagnostics.size());
    assertEquals("unknown_status", diagnostics.get(0).code());
    assertTrue(diagnostics.get(0).message().contains("status 7"));
  }

  @Test
  void aLibraryThatExportsNoXlsxGraftVersionThrowsNamingIt() {
    UnsatisfiedLinkError missing = new UnsatisfiedLinkError("Error looking up function");
    XlsxGraftLibrary api =
        (outPtr, outLen) -> {
          throw missing;
        };

    LibraryNotFoundException error =
        assertThrows(LibraryNotFoundException.class, () -> VersionCall.call(api, "/old.so"));

    assertEquals("/old.so exports no `xlsx_graft_version`", error.getMessage());
    assertSame(missing, error.getCause());
  }

  /** What the stand-in above stands for: JNA finds a missing function on its first call. */
  interface Missing extends com.sun.jna.Library {
    int xlsx_graft_nothing();
  }

  @Test
  void jnaThrowsUnsatisfiedLinkErrorOnTheFirstCallOfAMissingFunction() {
    Missing missing = Native.load(Repo.ENGINE_LIBRARY, Missing.class);

    assertThrows(UnsatisfiedLinkError.class, missing::xlsx_graft_nothing);
  }

  // A stand-in for xlsx_graft_version that returns `status` with a length
  // written, as a panic may leave one: a failure must not read it.
  private static Result<String> callReturning(int status) {
    XlsxGraftLibrary api =
        (PointerByReference outPtr, SizeTByReference outLen) -> {
          if (Native.SIZE_T_SIZE == 8) {
            outLen.getPointer().setLong(0, -1);
          } else {
            outLen.getPointer().setInt(0, -1);
          }
          return status;
        };
    return VersionCall.call(api, "/stand-in.so");
  }
}
