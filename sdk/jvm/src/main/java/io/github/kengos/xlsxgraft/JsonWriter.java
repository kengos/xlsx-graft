// Writes JSON (RFC 8259) from Java values, for the verify options: Java 8 has
// no writer of its own and the package depends on JNA alone.
//
//     Java                                       JSON
//     Map<String, ?>                             object, in its order
//     List<?>                                    array
//     String                                     string
//     Boolean                                    true, false
//     null                                       null
//     Byte, Short, Integer, Long, BigInteger,    number
//     BigDecimal, Float, Double
//
// Anything else throws IllegalArgumentException, as do a Map key that is
// not a String, a Float or Double that is NaN or infinite (JSON has no
// number for them) and nesting deeper than Json.MAX_DEPTH. The limit on the
// depth of the recursion stops it before it reaches the thread's stack limit,
// and stops a list or map that contains itself.

package io.github.kengos.xlsxgraft;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.List;
import java.util.Map;

final class JsonWriter {

  private static final char[] HEX = "0123456789abcdef".toCharArray();

  private final StringBuilder out = new StringBuilder();

  private JsonWriter() {}

  /**
   * The JSON text of {@code value}.
   *
   * @throws IllegalArgumentException the value holds a type, a number or a nesting JSON cannot
   *     write
   */
  static String write(Object value) {
    JsonWriter writer = new JsonWriter();
    writer.value(value, 0);
    return writer.out.toString();
  }

  private void value(Object value, int depth) {
    if (value == null) {
      out.append("null");
    } else if (value instanceof String) {
      string((String) value);
    } else if (value instanceof Boolean
        || value instanceof Byte
        || value instanceof Short
        || value instanceof Integer
        || value instanceof Long
        || value instanceof BigInteger
        || value instanceof BigDecimal) {
      out.append(value);
    } else if (value instanceof Float || value instanceof Double) {
      real((Number) value);
    } else if (value instanceof Map) {
      object((Map<?, ?>) value, nested(depth));
    } else if (value instanceof List) {
      array((List<?>) value, nested(depth));
    } else {
      throw new IllegalArgumentException(
          "a " + value.getClass().getName() + " cannot be written as JSON");
    }
  }

  private static int nested(int depth) {
    if (depth >= Json.MAX_DEPTH) {
      throw new IllegalArgumentException("the JSON nests deeper than " + Json.MAX_DEPTH);
    }
    return depth + 1;
  }

  private void real(Number number) {
    double value = number.doubleValue();
    if (Double.isNaN(value) || Double.isInfinite(value)) {
      throw new IllegalArgumentException("a number that is NaN or infinite cannot be written");
    }
    out.append(number);
  }

  private void object(Map<?, ?> map, int depth) {
    out.append('{');
    boolean first = true;
    for (Map.Entry<?, ?> entry : map.entrySet()) {
      if (!(entry.getKey() instanceof String)) {
        throw new IllegalArgumentException("a key of an object is not a String");
      }
      if (!first) {
        out.append(',');
      }
      first = false;
      string((String) entry.getKey());
      out.append(':');
      value(entry.getValue(), depth);
    }
    out.append('}');
  }

  private void array(List<?> list, int depth) {
    out.append('[');
    boolean first = true;
    for (Object element : list) {
      if (!first) {
        out.append(',');
      }
      first = false;
      value(element, depth);
    }
    out.append(']');
  }

  private void string(String text) {
    out.append('"');
    for (int at = 0; at < text.length(); at++) {
      char c = text.charAt(at);
      if (c == '"' || c == '\\') {
        out.append('\\').append(c);
      } else if (c < 0x20) {
        out.append("\\u00").append(HEX[c >> 4]).append(HEX[c & 15]);
      } else {
        out.append(c);
      }
    }
    out.append('"');
  }
}
