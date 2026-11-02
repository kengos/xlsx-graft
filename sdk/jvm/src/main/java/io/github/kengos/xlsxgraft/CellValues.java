// Turns the XlsxGraftValue array xlsx_graft_rows_next lends into Java values,
// copying everything out: the array and its text live only until the next
// call on the rows handle.
//
//     kind      Java
//     empty     null
//     string    String
//     number    Double, 3.0 included
//     boolean   Boolean
//     date      LocalDate
//     time      LocalTime
//     datetime  LocalDateTime
//     elapsed   Duration, not wrapped at a day
//     error     CellError
//
// A date, time or date-time is the workbook's wall-clock time with no time
// zone, and these types hold one as it is: none is applied.
//
// The struct is read at the header's offsets, the last two moved by the
// platform's pointer width (the header gives them for a 64-bit target).

package io.github.kengos.xlsxgraft;

import com.sun.jna.Native;
import com.sun.jna.Pointer;
import java.time.Duration;
import java.time.LocalDate;
import java.time.LocalDateTime;
import java.time.LocalTime;
import java.util.ArrayList;
import java.util.List;

final class CellValues {

  // XlsxGraftValue's fields. 48 bytes on a 64-bit target.
  static final int YEAR = 4;
  static final int MONTH = 6;
  static final int DAY = 7;
  static final int HOUR = 8;
  static final int MINUTE = 9;
  static final int SECOND = 10;
  static final int BOOLEAN = 11;
  static final int MILLISECOND = 12;
  static final int NUMBER = 16;
  static final int ELAPSED = 24;
  static final int TEXT = 32;
  static final int TEXT_LEN = TEXT + Native.POINTER_SIZE;
  // Aligned to 8, its double's and uint64_t's, or 4 where those are (i386):
  // the size is a multiple of 8 either way.
  static final int SIZE = Capi.align(TEXT_LEN + Native.SIZE_T_SIZE, 8);

  // The kinds this package knows, from 0: XLSX_GRAFT_VALUE_ERROR is the last.
  private static final int KINDS = 9;

  private CellValues() {}

  /**
   * The {@code count} values at {@code values}, a pointer to {@code XlsxGraftValue}.
   *
   * @throws UnknownValueException a value is of a kind this package does not know (an engine newer
   *     than the package), which is not guessed at
   */
  static List<Object> call(Pointer values, long count) throws UnknownValueException {
    int length = Math.toIntExact(count);
    List<Object> row = new ArrayList<>(length);
    for (int index = 0; index < length; index++) {
      Pointer value = values.share((long) index * SIZE);
      int kind = value.getInt(0);
      if (kind < 0 || kind >= KINDS) {
        throw new UnknownValueException(
            "the engine returned a value of kind "
                + (kind & 0xFFFFFFFFL)
                + ", which this package does not know");
      }
      row.add(value(kind, value));
    }
    return row;
  }

  private static Object value(int kind, Pointer value) {
    switch (kind) {
      case 0:
        return null;
      case 1:
        return text(value);
      case 2:
        return value.getDouble(NUMBER);
      case 3:
        return value.getByte(BOOLEAN) == 1;
      case 4:
        return date(value);
      case 5:
        return time(value);
      case 6:
        return LocalDateTime.of(date(value), time(value));
      case 7:
        return Duration.ofMillis(value.getLong(ELAPSED));
      default:
        return new CellError(text(value));
    }
  }

  private static String text(Pointer value) {
    return Capi.text(value.getPointer(TEXT), Capi.sizeT(value, TEXT_LEN));
  }

  private static LocalDate date(Pointer value) {
    return LocalDate.of(
        value.getShort(YEAR) & 0xFFFF, value.getByte(MONTH) & 0xFF, value.getByte(DAY) & 0xFF);
  }

  private static LocalTime time(Pointer value) {
    return LocalTime.of(
        value.getByte(HOUR) & 0xFF,
        value.getByte(MINUTE) & 0xFF,
        value.getByte(SECOND) & 0xFF,
        (value.getShort(MILLISECOND) & 0xFFFF) * 1_000_000);
  }

  /** A value of a kind this package does not know. */
  static final class UnknownValueException extends Exception {

    private static final long serialVersionUID = 1L;

    UnknownValueException(String message) {
      super(message);
    }
  }
}
