package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class VerifyRulesTest {

  @Test
  void rulesNoneGivesNothingToTheEngine() {
    assertTrue(VerifyRules.none().json().isEmpty());
  }

  @Test
  void eachRuleGivenIsInTheEnginesShapeAndNamesInOrder() {
    VerifyRules rules =
        VerifyRules.none()
            .failOn("warn")
            .allowParts(Arrays.asList("macro"))
            .allowFunctions(Arrays.asList("SEQUENCE"))
            .bestEffort(Arrays.asList("excel 2016"))
            .targets(Arrays.asList("excel >= 2016"));

    Map<String, Object> expected = new LinkedHashMap<>();
    expected.put("targets", Arrays.asList("excel >= 2016"));
    expected.put("bestEffort", Arrays.asList("excel 2016"));
    expected.put("functions", Collections.singletonMap("allow", Arrays.asList("SEQUENCE")));
    expected.put("parts", Collections.singletonMap("allow", Arrays.asList("macro")));
    expected.put("failOn", "warn");
    assertEquals(expected, rules.json());
    assertEquals(
        Arrays.asList("targets", "bestEffort", "functions", "parts", "failOn"),
        new ArrayList<>(rules.json().keySet()));
  }

  @Test
  void aRuleGivenAsNullIsNotGiven() {
    VerifyRules rules =
        VerifyRules.none()
            .targets(Arrays.asList("excel 2016"))
            .allowParts(Arrays.asList("macro"))
            .failOn("warn")
            .targets(null)
            .allowParts(null)
            .failOn(null)
            .bestEffort(null)
            .allowFunctions(null);

    assertTrue(rules.json().isEmpty());
  }

  @Test
  void aValueDoesNotChangeAndCopiesTheListsItIsGiven() {
    List<String> targets = new ArrayList<>(Arrays.asList("excel 2016"));
    VerifyRules none = VerifyRules.none();
    VerifyRules rules = none.targets(targets);
    targets.add("excel 2019");

    assertTrue(none.json().isEmpty());
    assertEquals(Arrays.asList("excel 2016"), rules.json().get("targets"));
  }
}
