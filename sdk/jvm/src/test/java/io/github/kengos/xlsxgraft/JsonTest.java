package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class JsonTest {

  @Test
  void readsEveryKindOfValue() throws Exception {
    Map<String, Object> expected = new LinkedHashMap<>();
    expected.put("b", Arrays.asList(true, false, null));
    expected.put("a", Collections.emptyMap());
    expected.put("n", Arrays.asList(0L, -12L, 1.5, -2e3, 1.0E2, 9.223372036854775808E18));
    expected.put("s", "q\"\\/\b\f\n\r\tA\u00e9\ud83d\ude00\udcff");
    expected.put("e", Collections.emptyList());

    Object value =
        parse(
            " {\"b\": [true, false, null], \"a\": {}, \"n\": [0, -12, 1.5, -2e3, 1.0E+2,"
                + " 9223372036854775808],\n\t\"s\": \"q\\\"\\\\\\/\\b\\f\\n\\r\\t\\u0041é\\ud83d\\ude00\\uDCFF\","
                + " \"e\": [ ]}\r\n");

    assertEquals(expected, value);
    assertEquals(
        Arrays.asList("b", "a", "n", "s", "e"),
        Arrays.asList(((Map<?, ?>) value).keySet().toArray()));
  }

  @Test
  void whatItReadsCannotBeChanged() throws Exception {
    Map<?, ?> object = (Map<?, ?>) parse("{\"l\": [1]}");

    assertThrows(UnsupportedOperationException.class, object::clear);
    assertThrows(UnsupportedOperationException.class, () -> ((List<?>) object.get("l")).clear());
  }

  @Test
  void readsAScalarAtTheTop() throws Exception {
    assertNull(parse("null"));
    assertEquals("x", parse("\"x\""));
  }

  @ParameterizedTest
  @ValueSource(
      strings = {
        "",
        "{",
        "{\"a\" 1}",
        "{1: 2}",
        "{\"a\": 1,}",
        "[1 2]",
        "[1,]",
        "tru",
        "nul",
        "01",
        "-",
        "1.",
        "1e",
        "1e+",
        "\"open",
        "\"\\x\"",
        "\"\\u12\"",
        "\"\\u12g4\"",
        "\"tab\there\"",
        "{} {}",
        "+1",
        "-a",
        "\"\\u\uff10041\"",
      })
  void throwsForTextThatIsNotJson(String text) {
    assertThrows(Json.UnreadableException.class, () -> parse(text));
  }

  @Test
  void throwsForBytesThatAreNotUtf8() {
    Json.UnreadableException error =
        assertThrows(
            Json.UnreadableException.class, () -> Json.parse(new byte[] {'"', (byte) 0xff, '"'}));

    assertEquals("the JSON is not UTF-8", error.getMessage());
  }

  @Test
  void throwsForNestingDeeperThanItsLimitAndReadsItsLimit() throws Exception {
    String deepest = repeat("[", Json.MAX_DEPTH) + repeat("]", Json.MAX_DEPTH);
    String deeper = repeat("[", Json.MAX_DEPTH + 1) + repeat("]", Json.MAX_DEPTH + 1);

    parse(deepest);
    Json.UnreadableException error =
        assertThrows(Json.UnreadableException.class, () -> parse(deeper));
    assertEquals("the JSON nests deeper than 64", error.getMessage());
  }

  @Test
  void saysWhereItStopped() {
    assertEquals(
        "unexpected x in the JSON at 1",
        assertThrows(Json.UnreadableException.class, () -> parse("[x]")).getMessage());
    assertEquals(
        "the JSON ends early",
        assertThrows(Json.UnreadableException.class, () -> parse("[1")).getMessage());
  }

  private static Object parse(String text) throws Json.UnreadableException {
    return Json.parse(text.getBytes(StandardCharsets.UTF_8));
  }

  private static String repeat(String text, int times) {
    StringBuilder out = new StringBuilder();
    for (int i = 0; i < times; i++) {
      out.append(text);
    }
    return out.toString();
  }
}
