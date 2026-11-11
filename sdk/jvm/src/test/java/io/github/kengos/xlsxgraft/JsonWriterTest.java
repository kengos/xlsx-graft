package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

class JsonWriterTest {

  @Test
  void writesEveryKindOfValueAndAMapInItsOrder() {
    Map<String, Object> value = new LinkedHashMap<>();
    value.put("b", Arrays.asList(true, false, null));
    value.put("a", Collections.emptyMap());
    value.put(
        "n",
        Arrays.<Object>asList(
            (byte) 1,
            (short) -2,
            3,
            -4L,
            new BigInteger("12345678901234567890"),
            new BigDecimal("1.50"),
            1.5f,
            -2.5d));
    value.put("s", "q\"\\/\n\t\u0001\u001fé😀");
    value.put("e", Collections.emptyList());

    assertEquals(
        "{\"b\":[true,false,null],\"a\":{},\"n\":[1,-2,3,-4,12345678901234567890,1.50,1.5,-2.5],"
            + "\"s\":\"q\\\"\\\\/\\u000a\\u0009\\u0001\\u001fé😀\",\"e\":[]}",
        JsonWriter.write(value));
  }

  @Test
  void writesAScalarAtTheTop() {
    assertEquals("null", JsonWriter.write(null));
    assertEquals("\"x\"", JsonWriter.write("x"));
  }

  @Test
  void refusesANaNAndAnInfinity() {
    for (Object number :
        new Object[] {Double.NaN, Double.POSITIVE_INFINITY, Float.NEGATIVE_INFINITY, Float.NaN}) {
      assertThrows(IllegalArgumentException.class, () -> JsonWriter.write(number));
    }
  }

  @Test
  void refusesATypeJsonHasNoValueFor() {
    IllegalArgumentException error =
        assertThrows(IllegalArgumentException.class, () -> JsonWriter.write(new AtomicInteger()));

    assertTrue(error.getMessage().contains("java.util.concurrent.atomic.AtomicInteger"));
    assertThrows(IllegalArgumentException.class, () -> JsonWriter.write('c'));
    assertThrows(IllegalArgumentException.class, () -> JsonWriter.write(new int[0]));
  }

  @Test
  void refusesAMapKeyThatIsNotAString() {
    assertThrows(
        IllegalArgumentException.class, () -> JsonWriter.write(Collections.singletonMap(1, 2)));
  }

  @Test
  void writesNestingUpToTheDepthTheReaderTakesAndRefusesDeeper() {
    assertEquals(
        repeat("[", Json.MAX_DEPTH) + repeat("]", Json.MAX_DEPTH),
        JsonWriter.write(nested(Json.MAX_DEPTH)));
    assertThrows(
        IllegalArgumentException.class, () -> JsonWriter.write(nested(Json.MAX_DEPTH + 1)));
    assertThrows(
        IllegalArgumentException.class,
        () -> JsonWriter.write(Collections.singletonMap("a", nested(Json.MAX_DEPTH))));
  }

  @Test
  void refusesAListThatContainsItself() {
    List<Object> loop = new ArrayList<>();
    loop.add(loop);

    assertThrows(IllegalArgumentException.class, () -> JsonWriter.write(loop));
  }

  private static List<Object> nested(int depth) {
    List<Object> list = new ArrayList<>();
    for (int level = 1; level < depth; level++) {
      list = new ArrayList<Object>(Collections.singletonList(list));
    }
    return list;
  }

  private static String repeat(String text, int times) {
    StringBuilder out = new StringBuilder();
    for (int count = 0; count < times; count++) {
      out.append(text);
    }
    return out.toString();
  }
}
