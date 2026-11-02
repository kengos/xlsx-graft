// Reads JSON (RFC 8259): the book's diagnostics are JSON, Java 8 has no
// reader of its own, and the package depends on JNA alone.
//
//     JSON      Java
//     object    Map<String, Object>, in its order
//     array     List<Object>
//     string    String
//     number    Long when it is a whole number written without a fraction
//               or an exponent that a long holds, else Double
//     true      Boolean
//     null      null
//
// Nothing read can be changed. Bytes that are not UTF-8, text that is not
// JSON, and nesting deeper than MAX_DEPTH throw UnreadableException; the
// depth keeps the recursion off the thread's stack limit.

package io.github.kengos.xlsxgraft;

import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

final class Json {

  static final int MAX_DEPTH = 64;

  private final String text;
  private int at;

  private Json(String text) {
    this.text = text;
  }

  /**
   * The value {@code bytes} hold.
   *
   * @throws UnreadableException the bytes are not UTF-8, or not JSON
   */
  static Object parse(byte[] bytes) throws UnreadableException {
    String text;
    try {
      text =
          StandardCharsets.UTF_8
              .newDecoder()
              .onMalformedInput(CodingErrorAction.REPORT)
              .onUnmappableCharacter(CodingErrorAction.REPORT)
              .decode(ByteBuffer.wrap(bytes))
              .toString();
    } catch (CharacterCodingException error) {
      throw new UnreadableException("the JSON is not UTF-8");
    }
    Json json = new Json(text);
    Object value = json.value(0);
    json.space();
    if (json.at != text.length()) {
      throw json.unexpected();
    }
    return value;
  }

  private Object value(int depth) throws UnreadableException {
    if (depth == MAX_DEPTH) {
      throw new UnreadableException("the JSON nests deeper than " + MAX_DEPTH);
    }
    space();
    char next = peek();
    if (next == '{') {
      return object(depth);
    }
    if (next == '[') {
      return array(depth);
    }
    if (next == '"') {
      return string();
    }
    if (next == '-' || (next >= '0' && next <= '9')) {
      return number();
    }
    if (word("true")) {
      return Boolean.TRUE;
    }
    if (word("false")) {
      return Boolean.FALSE;
    }
    if (word("null")) {
      return null;
    }
    throw unexpected();
  }

  private Map<String, Object> object(int depth) throws UnreadableException {
    at++;
    Map<String, Object> members = new LinkedHashMap<>();
    space();
    if (take('}')) {
      return Collections.unmodifiableMap(members);
    }
    do {
      space();
      if (peek() != '"') {
        throw unexpected();
      }
      String name = string();
      space();
      expect(':');
      members.put(name, value(depth + 1));
      space();
    } while (take(','));
    expect('}');
    return Collections.unmodifiableMap(members);
  }

  private List<Object> array(int depth) throws UnreadableException {
    at++;
    List<Object> elements = new ArrayList<>();
    space();
    if (take(']')) {
      return Collections.unmodifiableList(elements);
    }
    do {
      elements.add(value(depth + 1));
      space();
    } while (take(','));
    expect(']');
    return Collections.unmodifiableList(elements);
  }

  private String string() throws UnreadableException {
    at++;
    StringBuilder out = new StringBuilder();
    while (true) {
      char next = peek();
      at++;
      if (next == '"') {
        return out.toString();
      }
      if (next < 0x20) {
        throw new UnreadableException("a control character in a JSON string at " + (at - 1));
      }
      out.append(next == '\\' ? escape() : next);
    }
  }

  private char escape() throws UnreadableException {
    char next = peek();
    at++;
    switch (next) {
      case '"':
      case '\\':
      case '/':
        return next;
      case 'b':
        return '\b';
      case 'f':
        return '\f';
      case 'n':
        return '\n';
      case 'r':
        return '\r';
      case 't':
        return '\t';
      case 'u':
        return unicode();
      default:
        at--;
        throw unexpected();
    }
  }

  // A lone surrogate is kept as it is, as a Java string may hold one.
  private char unicode() throws UnreadableException {
    if (at + 4 > text.length()) {
      throw unexpected();
    }
    int code = 0;
    for (int end = at + 4; at < end; at++) {
      // ASCII digits alone: Character.digit takes any script's.
      char written = text.charAt(at);
      int digit = written < 0x80 ? Character.digit(written, 16) : -1;
      if (digit < 0) {
        throw unexpected();
      }
      code = code * 16 + digit;
    }
    return (char) code;
  }

  private Object number() throws UnreadableException {
    int start = at;
    take('-');
    if (!take('0')) {
      digits();
    }
    boolean whole = true;
    if (take('.')) {
      whole = false;
      digits();
    }
    if (take('e') || take('E')) {
      whole = false;
      if (!take('+')) {
        take('-');
      }
      digits();
    }
    String written = text.substring(start, at);
    if (whole) {
      try {
        return Long.parseLong(written);
      } catch (NumberFormatException tooLarge) {
        // Past what a long holds: a double, as a fraction is.
      }
    }
    return Double.parseDouble(written);
  }

  // One digit or more.
  private void digits() throws UnreadableException {
    char next = peek();
    if (next < '0' || next > '9') {
      throw unexpected();
    }
    while (at < text.length() && text.charAt(at) >= '0' && text.charAt(at) <= '9') {
      at++;
    }
  }

  private void space() {
    while (at < text.length() && " \t\r\n".indexOf(text.charAt(at)) >= 0) {
      at++;
    }
  }

  private boolean word(String word) {
    if (text.startsWith(word, at)) {
      at += word.length();
      return true;
    }
    return false;
  }

  private boolean take(char expected) {
    if (at < text.length() && text.charAt(at) == expected) {
      at++;
      return true;
    }
    return false;
  }

  private void expect(char expected) throws UnreadableException {
    if (!take(expected)) {
      throw unexpected();
    }
  }

  private char peek() throws UnreadableException {
    if (at >= text.length()) {
      throw new UnreadableException("the JSON ends early");
    }
    return text.charAt(at);
  }

  private UnreadableException unexpected() {
    return at >= text.length()
        ? new UnreadableException("the JSON ends early")
        : new UnreadableException("unexpected " + text.charAt(at) + " in the JSON at " + at);
  }

  /** Text that is not JSON, or JSON not of the shape asked for. */
  static final class UnreadableException extends Exception {

    private static final long serialVersionUID = 1L;

    UnreadableException(String message) {
      super(message);
    }
  }
}
