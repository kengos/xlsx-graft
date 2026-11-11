package io.github.kengos.xlsxgraft;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * Verifies workbooks through the C ABI, under the host's policy (given to the constructor, and
 * checked there) and the rules of the caller (given to each call).
 *
 * <p>A verifier does not change once it is made, so one may be shared by any number of threads, and
 * their calls may run at once.
 *
 * <pre>{@code
 * Map<String, Object> policy = new LinkedHashMap<>();
 * policy.put("maxFormulaBytes", 32768);
 * Verifier verifier = new Verifier(policy);
 *
 * VerifyRules rules =
 *     VerifyRules.none().targets(Arrays.asList("excel >= 2016")).allowParts(Arrays.asList("macro"));
 * Result<Void> result = verifier.verify(Paths.get("book.xlsx"), rules);
 * for (Diagnostic d : result.diagnostics()) {
 *   System.err.println(d.severity() + " " + d.code() + ": " + d.message());
 * }
 * if (result.error() != null) {
 *   System.err.println(result.error().message());
 * }
 * }</pre>
 *
 * <p>The result of a verification is a {@code Result<Void>}: it succeeds when the engine's {@code
 * ok} is true, and it has no value. {@link Result#error()} is set when the file cannot be read
 * ({@code io}) or the call itself failed ({@code panic}, {@code null_arg}, {@code unknown_status},
 * {@code diagnostics_unreadable}). A fault of the rules is a {@code rules_invalid} or {@code
 * target_invalid} diagnostic, and the workbook is not opened.
 */
public final class Verifier {

  /** The most JSON, in UTF-8 bytes, the engine takes as the policy and the rules together. */
  static final int MAX_OPTIONS_BYTES = 2 << 20;

  private final VerifyCall call;

  // The policy's JSON, as the engine checked it, or "null" for the engine's defaults.
  private final String policy;

  /** A verifier under the engine's default policy, over the engine library the package finds. */
  public Verifier() {
    this(null);
  }

  /**
   * A verifier under {@code policy}, over the engine library the package finds.
   *
   * @param policy the host's policy: the engine's limits, {@code failOn}, {@code denyScheduled} and
   *     {@code forbid}, as a JSON object in a Map (see {@link #Verifier(Map, String)}); null for
   *     the engine's defaults
   * @throws PolicyInvalidException the engine does not take the policy
   * @throws IllegalArgumentException the policy holds a value JSON cannot write
   * @throws LibraryNotFoundException no engine library was found or loaded, or it lacks a function
   */
  public Verifier(Map<String, ?> policy) {
    this(policy, (String) null);
  }

  /**
   * As {@link #Verifier(Map)}, from the engine library at {@code library}.
   *
   * <p>The keys are the engine's, in camelCase ({@code maxEntries}), and are not changed. A value
   * is a Map, a List, a String, a Boolean, null or a number ({@link Byte}, {@link Short}, {@link
   * Integer}, {@link Long}, {@link java.math.BigInteger}, {@link java.math.BigDecimal}, {@link
   * Float} or {@link Double}). The policy is written to JSON now, so changing the Map after changes
   * nothing.
   *
   * @param policy the host's policy; null for the engine's defaults
   * @param library the engine library's path, used when {@code XLSX_GRAFT_LIBRARY} is not set; may
   *     be null
   * @throws PolicyInvalidException the engine does not take the policy
   * @throws IllegalArgumentException the policy holds a value JSON cannot write
   * @throws LibraryNotFoundException no engine library was found or loaded, or it lacks a function
   */
  public Verifier(Map<String, ?> policy, String library) {
    this(policy, library, System.getenv());
  }

  Verifier(Map<String, ?> policy, String library, Map<String, String> env) {
    this(policy, new VerifyCall(Library.open(library, env)));
  }

  // The verify functions given, for a test that answers in place of the engine.
  Verifier(Map<String, ?> policy, VerifyCall call) {
    this.call = call;
    this.policy = policy == null ? "null" : checked(JsonWriter.write(policy));
  }

  /**
   * Verifies the workbook at {@code path}, under the verifier's policy and no rules.
   *
   * @param path the workbook's path, a relative one taken from the working directory as Java takes
   *     it ({@code user.dir})
   * @return the result (see the class description)
   * @throws NullPointerException {@code path} is null
   */
  public Result<Void> verify(Path path) {
    return verify(path, VerifyRules.none());
  }

  /**
   * Verifies the workbook at {@code path}, under the verifier's policy and {@code rules}.
   *
   * <p>A path holding an unpaired surrogate, which only a Windows file name can hold and which
   * cannot be encoded as UTF-8, is not given to the engine: the package reads the file and verifies
   * its bytes, so a file it cannot read is an {@code io} error, and the rules are not checked.
   *
   * @param path the workbook's path
   * @param rules the rules of this call
   * @return the result (see the class description)
   * @throws NullPointerException {@code path} or {@code rules} is null
   * @throws IllegalArgumentException the policy and the rules make more than 2 MiB of JSON, or a
   *     rule holds text that cannot be encoded as UTF-8 (an unpaired surrogate)
   */
  public Result<Void> verify(Path path, VerifyRules rules) {
    Objects.requireNonNull(path, "path");
    byte[] options = options(rules);
    byte[] encoded = Capi.utf8(path.toAbsolutePath().toString());
    return encoded == null ? read(path, options) : report(call.path(encoded, options));
  }

  /**
   * Verifies the workbook in {@code data}, under the verifier's policy and no rules.
   *
   * @param data the workbook's bytes
   * @return the result (see the class description)
   * @throws NullPointerException {@code data} is null
   */
  public Result<Void> verifyBytes(byte[] data) {
    return verifyBytes(data, VerifyRules.none());
  }

  /**
   * Verifies the workbook in {@code data}, under the verifier's policy and {@code rules}.
   *
   * <p>The package does not copy {@code data}: the call reads the array while it runs (JNA passes
   * it through JNI, which may copy or pin it), so do not change the array until the call returns.
   *
   * @param data the workbook's bytes
   * @param rules the rules of this call
   * @return the result (see the class description)
   * @throws NullPointerException {@code data} or {@code rules} is null
   * @throws IllegalArgumentException the policy and the rules make more than 2 MiB of JSON, or a
   *     rule holds text that cannot be encoded as UTF-8 (an unpaired surrogate)
   */
  public Result<Void> verifyBytes(byte[] data, VerifyRules rules) {
    Objects.requireNonNull(data, "data");
    return report(call.bytes(data, options(rules)));
  }

  // The policy's JSON, when the engine takes it.
  private String checked(String json) {
    Result<Void> result = report(call.policy(utf8(json)));
    if (result.failed()) {
      List<Diagnostic> faults = new ArrayList<>();
      if (result.error() != null) {
        faults.add(result.error());
      }
      faults.addAll(result.diagnostics());
      throw new PolicyInvalidException(faults);
    }
    return json;
  }

  // The options JSON as UTF-8: the policy and the rules the caller gave, by
  // the engine's names.
  private byte[] options(VerifyRules rules) {
    Objects.requireNonNull(rules, "rules");
    byte[] options =
        utf8("{\"policy\":" + policy + ",\"rules\":" + JsonWriter.write(rules.json()) + "}");
    if (options.length > MAX_OPTIONS_BYTES) {
      throw new IllegalArgumentException(
          "the policy and the rules are "
              + options.length
              + " bytes of JSON; the engine takes "
              + MAX_OPTIONS_BYTES);
    }
    return options;
  }

  private static byte[] utf8(String json) {
    byte[] bytes = Capi.utf8(json);
    if (bytes == null) {
      throw new IllegalArgumentException(
          "the policy or a rule holds text that cannot be encoded as UTF-8 (an unpaired surrogate)");
    }
    return bytes;
  }

  private static Result<Void> report(Capi.Lent lent) {
    return VerifyReport.call(lent.status, lent.bytes);
  }

  // The engine takes no path that cannot be encoded as UTF-8, so the package
  // reads the file and verifies its bytes; a file that cannot be read is an `io` failure.
  Result<Void> read(Path path, byte[] options) {
    byte[] data;
    try {
      data = Files.readAllBytes(path);
    } catch (IOException unreadable) {
      return Result.failure(
          Collections.<Diagnostic>emptyList(),
          StatusDiagnostic.error("io", "the file could not be read: " + reason(unreadable)));
    }
    return report(call.bytes(data, options));
  }

  // The message of `failure`, or its class name when it has none.
  static String reason(IOException failure) {
    return failure.getMessage() == null ? failure.getClass().getName() : failure.getMessage();
  }
}
