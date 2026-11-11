// The handle each verify function makes is closed whatever comes after it.
// Each test gives VerifyCall a Proxy of XlsxGraftLibrary that returns a status
// no workbook makes the engine return (PANIC, an unknown status), writes no
// handle, or throws between the call and the close.

package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.sun.jna.Memory;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import org.junit.jupiter.api.Test;

class VerifyCallTest {

  private static final byte[] OPTIONS = new byte[] {'{', '}'};

  @Test
  void aStatusOtherThanOkWritesNoHandleSoNothingIsClosed() {
    Fake fake = new Fake();
    fake.makeStatus = 2;
    fake.writesHandle = false;

    Capi.Lent lent = fake.call().path(new byte[] {'a'}, OPTIONS);

    assertEquals(2, lent.status);
    assertEquals(0, lent.bytes.length);
    assertEquals(0, fake.closed.size());
  }

  @Test
  void aResultIsCopiedOutAndTheHandleClosedOnceForEachFunction() {
    byte[] json = "{\"ok\":true}".getBytes(StandardCharsets.UTF_8);
    Fake fake = new Fake();
    fake.json = json;
    VerifyCall call = fake.call();

    Capi.Lent path = call.path(new byte[] {'a'}, OPTIONS);
    Capi.Lent bytes = call.bytes(new byte[0], OPTIONS);
    Capi.Lent policy = call.policy(OPTIONS);

    for (Capi.Lent lent : new Capi.Lent[] {path, bytes, policy}) {
      assertEquals(0, lent.status);
      assertArrayEquals(json, lent.bytes);
    }
    assertEquals(Collections.nCopies(3, fake.handle), fake.closed);
    assertEquals(
        Arrays.asList(
            "xlsx_graft_verify_path", "xlsx_graft_verify_bytes", "xlsx_graft_policy_check"),
        fake.made);
  }

  @Test
  void aResultThatCannotBeLentGivesItsStatusAndTheHandleIsClosed() {
    Fake fake = new Fake();
    fake.resultStatus = 2;

    Capi.Lent lent = fake.call().bytes(new byte[] {1}, OPTIONS);

    assertEquals(2, lent.status);
    assertEquals(0, lent.bytes.length);
    assertEquals(Collections.singletonList(fake.handle), fake.closed);
  }

  @Test
  void aResultOfNoBytesGivesNoBytes() {
    Fake fake = new Fake();
    fake.json = new byte[0];

    Capi.Lent lent = fake.call().policy(OPTIONS);

    assertEquals(0, lent.status);
    assertEquals(0, lent.bytes.length);
    assertEquals(Collections.singletonList(fake.handle), fake.closed);
  }

  @Test
  void anExceptionBetweenTheCallAndTheCloseStillClosesTheHandle() {
    Fake fake = new Fake();
    fake.resultThrows = new IllegalStateException("interrupted");

    IllegalStateException error =
        assertThrows(
            IllegalStateException.class, () -> fake.call().path(new byte[] {'a'}, OPTIONS));

    assertSame(fake.resultThrows, error);
    assertEquals(Collections.singletonList(fake.handle), fake.closed);
  }

  @Test
  void theEngineLibraryExportsEveryVerifyFunction() {
    Library library = Library.open(Repo.ENGINE_LIBRARY, Collections.<String, String>emptyMap());

    library.require(VerifyCall.FUNCTIONS);
    new VerifyCall(library);
  }

  /** A Proxy of XlsxGraftLibrary whose functions return what the test sets. */
  static final class Fake implements InvocationHandler {

    final Pointer handle = new Pointer(8);
    final List<Pointer> closed = new ArrayList<>();
    final List<String> made = new ArrayList<>();

    int makeStatus = 0;
    boolean writesHandle = true;
    int resultStatus = 0;
    byte[] json = new byte[0];
    RuntimeException resultThrows;

    // Held so the memory the result lends lives through the test.
    private Memory lent;

    VerifyCall call() {
      return new VerifyCall(
          (XlsxGraftLibrary)
              Proxy.newProxyInstance(
                  XlsxGraftLibrary.class.getClassLoader(),
                  new Class<?>[] {XlsxGraftLibrary.class},
                  this));
    }

    @Override
    public Object invoke(Object proxy, java.lang.reflect.Method method, Object[] args) {
      switch (method.getName()) {
        case "xlsx_graft_verify_path":
        case "xlsx_graft_verify_bytes":
        case "xlsx_graft_policy_check":
          made.add(method.getName());
          if (writesHandle) {
            ((PointerByReference) args[args.length - 1]).setValue(handle);
          }
          return makeStatus;
        case "xlsx_graft_verify_result":
          if (resultThrows != null) {
            throw resultThrows;
          }
          if (resultStatus == 0) {
            lent = new Memory(Math.max(json.length, 1));
            lent.write(0, json, 0, json.length);
            ((PointerByReference) args[1]).setValue(lent);
            Pointer length = ((SizeTByReference) args[2]).getPointer();
            if (Native.SIZE_T_SIZE == 8) {
              length.setLong(0, json.length);
            } else {
              length.setInt(0, json.length);
            }
          }
          return resultStatus;
        case "xlsx_graft_verify_close":
          closed.add((Pointer) args[0]);
          return 0;
        default:
          throw new UnsupportedOperationException(method.getName());
      }
    }
  }
}
