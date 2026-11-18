// The handle each view function makes is closed whatever comes after it, and the message of a
// refused value is read right after the call. Some tests call the engine library; the others give
// ViewCall a Proxy of XlsxGraftLibrary that returns a status no workbook makes the engine return
// (PANIC, an unknown status), writes no handle, or throws between the call and the close.

package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.sun.jna.Memory;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;
import java.io.IOException;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import org.junit.jupiter.api.Test;

class ViewCallTest {

  private static final Library LIBRARY =
      Library.open(Repo.ENGINE_LIBRARY, Collections.<String, String>emptyMap());
  private static final ViewCall CALL = new ViewCall(LIBRARY);
  private static final Path MERGED = Repo.ROOT.resolve("fixtures/merged.xlsx");
  private static final byte[] OPTIONS = new byte[] {'{', '}'};

  private static String text(byte[] json) {
    return new String(json, StandardCharsets.UTF_8);
  }

  @Test
  void treePathAndTreeBytesGiveTheStatusAndTheViewsJsonCopiedOut() throws IOException {
    ViewCall.Outcome path = CALL.treePath(MERGED.toString().getBytes(StandardCharsets.UTF_8));
    ViewCall.Outcome bytes = CALL.treeBytes(Files.readAllBytes(MERGED));
    ViewCall.Outcome empty = CALL.treeBytes(new byte[0]);

    assertEquals(0, path.status);
    assertTrue(text(path.json).startsWith("{\"ok\":true"), text(path.json));
    assertNull(path.message);
    assertEquals(0, bytes.status);
    assertTrue(text(bytes.json).contains("\"/xl/workbook.xml\""), text(bytes.json));
    assertEquals(0, empty.status);
    assertTrue(text(empty.json).startsWith("{\"ok\":false"), text(empty.json));
  }

  @Test
  void sheetPathAndSheetBytesGiveTheCellsTheOptionsAskFor() throws IOException {
    byte[] data = Files.readAllBytes(MERGED);

    ViewCall.Outcome path =
        CALL.sheetPath(
            MERGED.toString().getBytes(StandardCharsets.UTF_8),
            "{\"maxRows\":2}".getBytes(StandardCharsets.UTF_8));
    ViewCall.Outcome bytes =
        CALL.sheetBytes(data, "{\"maxColumns\":2}".getBytes(StandardCharsets.UTF_8));
    ViewCall.Outcome defaults = CALL.sheetBytes(data, new byte[0]);

    assertEquals(0, path.status);
    assertTrue(text(path.json).contains("\"shown\":\"A1:E2\""), text(path.json));
    assertTrue(text(bytes.json).contains("\"shown\":\"A1:B8\""), text(bytes.json));
    assertTrue(text(defaults.json).contains("\"shown\":\"A1:E8\""), text(defaults.json));
  }

  @Test
  void aValueTheEngineRefusesGivesRequestInvalidAndItsMessageAndNoJson() {
    ViewCall.Outcome outcome =
        CALL.sheetBytes(new byte[0], "{\"maxRows\":0}".getBytes(StandardCharsets.UTF_8));

    assertEquals(StatusDiagnostic.REQUEST_INVALID, outcome.status);
    assertNull(outcome.json);
    assertTrue(outcome.message.contains("maxRows"), outcome.message);
  }

  @Test
  void optionsThatAreNotJsonGiveInvalidArgAndNeitherJsonNorMessage() {
    ViewCall.Outcome outcome =
        CALL.sheetBytes(new byte[0], "not json".getBytes(StandardCharsets.UTF_8));

    assertEquals(StatusDiagnostic.INVALID_ARG, outcome.status);
    assertNull(outcome.json);
    assertNull(outcome.message);
  }

  @Test
  void aStatusOtherThanOkWritesNoHandleSoNothingIsClosed() {
    Fake fake = new Fake();
    fake.makeStatus = 2;
    fake.writesHandle = false;

    ViewCall.Outcome outcome = fake.call().treePath(new byte[] {'a'});

    assertEquals(2, outcome.status);
    assertNull(outcome.json);
    assertNull(outcome.message);
    assertEquals(0, fake.closed.size());
  }

  @Test
  void aResultIsCopiedOutAndTheHandleClosedOnceForEachFunction() {
    byte[] json = "{\"ok\":true}".getBytes(StandardCharsets.UTF_8);
    Fake fake = new Fake();
    fake.json = json;
    ViewCall call = fake.call();

    ViewCall.Outcome[] outcomes = {
      call.treePath(new byte[] {'a'}),
      call.treeBytes(new byte[0]),
      call.sheetPath(new byte[] {'a'}, OPTIONS),
      call.sheetBytes(new byte[0], OPTIONS)
    };

    for (ViewCall.Outcome outcome : outcomes) {
      assertEquals(0, outcome.status);
      assertArrayEquals(json, outcome.json);
      assertNull(outcome.message);
    }
    assertEquals(Collections.nCopies(4, fake.handle), fake.closed);
    assertEquals(
        Arrays.asList(
            "xlsx_graft_tree_path",
            "xlsx_graft_tree_bytes",
            "xlsx_graft_describe_sheet_path",
            "xlsx_graft_describe_sheet_bytes"),
        fake.made);
  }

  @Test
  void aResultThatCannotBeReadGivesItsStatusAndTheHandleIsClosed() {
    Fake fake = new Fake();
    fake.resultStatus = 2;

    ViewCall.Outcome outcome = fake.call().treeBytes(new byte[] {1});

    assertEquals(2, outcome.status);
    assertNull(outcome.json);
    assertEquals(Collections.singletonList(fake.handle), fake.closed);
  }

  @Test
  void aResultOfNoBytesGivesNoBytes() {
    Fake fake = new Fake();
    fake.json = new byte[0];

    ViewCall.Outcome outcome = fake.call().sheetBytes(new byte[0], OPTIONS);

    assertEquals(0, outcome.status);
    assertEquals(0, outcome.json.length);
    assertEquals(Collections.singletonList(fake.handle), fake.closed);
  }

  @Test
  void anExceptionBetweenTheCallAndTheCloseStillClosesTheHandle() {
    Fake fake = new Fake();
    fake.resultThrows = new IllegalStateException("interrupted");

    IllegalStateException error =
        assertThrows(IllegalStateException.class, () -> fake.call().treePath(new byte[] {'a'}));

    assertSame(fake.resultThrows, error);
    assertEquals(Collections.singletonList(fake.handle), fake.closed);
  }

  // The engine keeps the message per thread until the next failure on it, so a thread the test's
  // executor has just made has no message yet: error_message writes an empty one.
  @Test
  void requestInvalidFromTheFakeOnAThreadWithNoMessageGivesAnEmptyMessageAndTheFixedSentence()
      throws Exception {
    Fake fake = new Fake();
    fake.makeStatus = StatusDiagnostic.REQUEST_INVALID;
    fake.writesHandle = false;
    ExecutorService pool = Executors.newSingleThreadExecutor();
    try {
      ViewCall.Outcome outcome =
          pool.submit(() -> fake.call().sheetBytes(new byte[0], OPTIONS)).get();

      assertEquals(StatusDiagnostic.REQUEST_INVALID, outcome.status);
      assertNull(outcome.json);
      assertEquals("", outcome.message);
      IllegalArgumentException error =
          assertThrows(
              IllegalArgumentException.class, () -> ViewReport.call(outcome, view -> "unused"));
      assertEquals(
          "the engine refused a value of the options (status 11) and gave no reason",
          error.getMessage());
    } finally {
      pool.shutdown();
    }
  }

  @Test
  void theEngineLibraryExportsEveryViewFunction() {
    LIBRARY.require(ViewCall.FUNCTIONS);
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

    // Kept so that the memory view_result returns is not freed while the test runs.
    private Memory result;

    ViewCall call() {
      return new ViewCall(
          (XlsxGraftLibrary)
              Proxy.newProxyInstance(
                  XlsxGraftLibrary.class.getClassLoader(),
                  new Class<?>[] {XlsxGraftLibrary.class},
                  this),
          LIBRARY.function("xlsx_graft_error_message"));
    }

    @Override
    public Object invoke(Object proxy, java.lang.reflect.Method method, Object[] args) {
      switch (method.getName()) {
        case "xlsx_graft_tree_path":
        case "xlsx_graft_tree_bytes":
        case "xlsx_graft_describe_sheet_path":
        case "xlsx_graft_describe_sheet_bytes":
          made.add(method.getName());
          if (writesHandle) {
            ((PointerByReference) args[args.length - 1]).setValue(handle);
          }
          return makeStatus;
        case "xlsx_graft_view_result":
          if (resultThrows != null) {
            throw resultThrows;
          }
          if (resultStatus == 0) {
            result = new Memory(Math.max(json.length, 1));
            result.write(0, json, 0, json.length);
            ((PointerByReference) args[1]).setValue(result);
            Pointer length = ((SizeTByReference) args[2]).getPointer();
            if (Native.SIZE_T_SIZE == 8) {
              length.setLong(0, json.length);
            } else {
              length.setInt(0, json.length);
            }
          }
          return resultStatus;
        case "xlsx_graft_view_close":
          closed.add((Pointer) args[0]);
          return 0;
        default:
          throw new UnsupportedOperationException(method.getName());
      }
    }
  }
}
