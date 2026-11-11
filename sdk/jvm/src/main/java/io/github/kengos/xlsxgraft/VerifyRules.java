package io.github.kengos.xlsxgraft;

import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * The rules of one {@link Verifier} call: what the caller asks of the workbook, on top of the
 * verifier's policy. Each rule is optional, and a rule that is not given (or given as null) is not
 * passed to the engine. A value of this class does not change: each method returns a new one.
 *
 * <pre>{@code
 * VerifyRules rules =
 *     VerifyRules.none().targets(Arrays.asList("excel >= 2016")).allowParts(Arrays.asList("macro"));
 * }</pre>
 *
 * <p>The package does not check a rule's value: a rule the engine does not take is a {@code
 * rules_invalid} or {@code target_invalid} diagnostic of the result.
 */
public final class VerifyRules {

  private static final VerifyRules NONE = new VerifyRules(null, null, null, null, null);

  private final List<String> targets;
  private final List<String> bestEffort;
  private final List<String> allowFunctions;
  private final List<String> allowParts;
  private final String failOn;

  private VerifyRules(
      List<String> targets,
      List<String> bestEffort,
      List<String> allowFunctions,
      List<String> allowParts,
      String failOn) {
    this.targets = targets;
    this.bestEffort = bestEffort;
    this.allowFunctions = allowFunctions;
    this.allowParts = allowParts;
    this.failOn = failOn;
  }

  /**
   * The rules with no rule given.
   *
   * @return rules that pass nothing to the engine
   */
  public static VerifyRules none() {
    return NONE;
  }

  /**
   * These rules with the versions the workbook must run in, such as {@code "excel >= 2016"}; a
   * problem with one is an error.
   *
   * @param targets the versions, copied now; null to give none
   * @return the new rules
   */
  public VerifyRules targets(List<String> targets) {
    return new VerifyRules(copy(targets), bestEffort, allowFunctions, allowParts, failOn);
  }

  /**
   * These rules with the versions the workbook should run in; a problem with one is a warning.
   *
   * @param bestEffort the versions, copied now; null to give none
   * @return the new rules
   */
  public VerifyRules bestEffort(List<String> bestEffort) {
    return new VerifyRules(targets, copy(bestEffort), allowFunctions, allowParts, failOn);
  }

  /**
   * These rules with the functions the verifier refuses by default that the call allows, named
   * without an {@code _xlfn.} prefix.
   *
   * @param allowFunctions the function names, copied now; null to give none
   * @return the new rules
   */
  public VerifyRules allowFunctions(List<String> allowFunctions) {
    return new VerifyRules(targets, bestEffort, copy(allowFunctions), allowParts, failOn);
  }

  /**
   * These rules with the kinds of part the verifier refuses by default that the call allows: {@code
   * macro}, {@code external_link}, {@code dde}, {@code ole}, {@code activex}, {@code
   * data_connection} or {@code web_publish}.
   *
   * @param allowParts the kinds, copied now; null to give none
   * @return the new rules
   */
  public VerifyRules allowParts(List<String> allowParts) {
    return new VerifyRules(targets, bestEffort, allowFunctions, copy(allowParts), failOn);
  }

  /**
   * These rules with the severity that fails the verification: {@code "warn"} fails on a warning
   * too. The stricter of this and the policy's {@code failOn} counts.
   *
   * @param failOn {@code "error"} or {@code "warn"}; null to give none
   * @return the new rules
   */
  public VerifyRules failOn(String failOn) {
    return new VerifyRules(targets, bestEffort, allowFunctions, allowParts, failOn);
  }

  /** The rules the way the engine names them: the rules given, in the shape of its JSON. */
  Map<String, Object> json() {
    Map<String, Object> rules = new LinkedHashMap<>();
    put(rules, "targets", targets);
    put(rules, "bestEffort", bestEffort);
    if (allowFunctions != null) {
      rules.put("functions", Collections.singletonMap("allow", allowFunctions));
    }
    if (allowParts != null) {
      rules.put("parts", Collections.singletonMap("allow", allowParts));
    }
    put(rules, "failOn", failOn);
    return rules;
  }

  private static void put(Map<String, Object> rules, String name, Object value) {
    if (value != null) {
      rules.put(name, value);
    }
  }

  private static List<String> copy(List<String> values) {
    return values == null ? null : Collections.unmodifiableList(new ArrayList<>(values));
  }
}
