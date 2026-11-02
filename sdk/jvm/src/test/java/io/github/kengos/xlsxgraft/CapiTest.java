package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.Structure;
import java.nio.charset.StandardCharsets;
import java.util.Collections;
import org.junit.jupiter.api.Test;

class CapiTest {

  /** XlsxGraftValue as JNA lays a C struct out, to check the offsets read by hand. */
  @Structure.FieldOrder({
    "kind",
    "year",
    "month",
    "day",
    "hour",
    "minute",
    "second",
    "bool",
    "millisecond",
    "reserved",
    "number",
    "elapsed",
    "text",
    "textLen"
  })
  public static final class CValue extends Structure {
    public int kind;
    public short year;
    public byte month;
    public byte day;
    public byte hour;
    public byte minute;
    public byte second;
    public byte bool;
    public short millisecond;
    public short reserved;
    public double number;
    public long elapsed;
    public Pointer text;
    public SizeT textLen;

    int offset(String field) {
      return fieldOffset(field);
    }
  }

  /** XlsxGraftSheet as JNA lays a C struct out. */
  @Structure.FieldOrder({"name", "nameLen", "kind", "state"})
  public static final class CSheet extends Structure {
    public Pointer name;
    public SizeT nameLen;
    public int kind;
    public int state;

    int offset(String field) {
      return fieldOffset(field);
    }
  }

  @Test
  void readsTheStructsAtTheOffsetsACompilerLaysThemOutAt() {
    CValue value = new CValue();
    CSheet sheet = new CSheet();

    assertEquals(value.size(), CellValues.SIZE);
    assertEquals(value.offset("year"), CellValues.YEAR);
    assertEquals(value.offset("month"), CellValues.MONTH);
    assertEquals(value.offset("day"), CellValues.DAY);
    assertEquals(value.offset("hour"), CellValues.HOUR);
    assertEquals(value.offset("minute"), CellValues.MINUTE);
    assertEquals(value.offset("second"), CellValues.SECOND);
    assertEquals(value.offset("bool"), CellValues.BOOLEAN);
    assertEquals(value.offset("millisecond"), CellValues.MILLISECOND);
    assertEquals(value.offset("number"), CellValues.NUMBER);
    assertEquals(value.offset("elapsed"), CellValues.ELAPSED);
    assertEquals(value.offset("text"), CellValues.TEXT);
    assertEquals(value.offset("textLen"), CellValues.TEXT_LEN);
    assertEquals(sheet.size(), Capi.SHEET_SIZE);
    assertEquals(sheet.offset("nameLen"), Capi.SHEET_NAME_LEN);
    assertEquals(sheet.offset("kind"), Capi.SHEET_KIND);
    assertEquals(sheet.offset("state"), Capi.SHEET_STATE);
    if (Native.POINTER_SIZE == 8) {
      assertEquals(48, CellValues.SIZE);
      assertEquals(24, Capi.SHEET_SIZE);
    }
  }

  @Test
  void utf8EncodesTextAndRefusesALoneSurrogate() {
    assertArrayEquals("é".getBytes(StandardCharsets.UTF_8), Capi.utf8("é"));
    assertNull(Capi.utf8("a\udcff"));
  }

  @Test
  void aRowsCallOnNoHandleIsRefusedWritingNothing() {
    Capi capi = new Capi(Library.open(Repo.ENGINE_LIBRARY, Collections.<String, String>emptyMap()));

    Capi.Next next = capi.rowsNext(null);

    assertEquals(1, next.status);
    assertEquals(0, next.number);
    assertNull(next.values);
    assertEquals(0, next.count);
  }

  @Test
  void aSizeTIsZeroUnlessGivenAValue() {
    assertEquals(0, new SizeT().longValue());
    assertEquals(7, new SizeT(7).longValue());
  }

  @Test
  void aLibraryThatLacksAFunctionThrowsNamingIt() {
    Library library = Library.open(Repo.ENGINE_LIBRARY, Collections.<String, String>emptyMap());

    LibraryNotFoundException error =
        assertThrows(LibraryNotFoundException.class, () -> library.require("xlsx_graft_nothing"));

    assertEquals(library.path() + " exports no `xlsx_graft_nothing`", error.getMessage());
    assertTrue(error.getCause() instanceof UnsatisfiedLinkError);
    library.require(Capi.FUNCTIONS);
  }
}
