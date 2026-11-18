package io.github.kengos.xlsxgraft;

/**
 * Where a relationship points, as the engine's {@code tree} shows it: a part of the package, or a
 * target outside it. One of {@link #part()} and {@link #external()} is null.
 */
public final class TreeTarget {

  private final String part;
  private final String external;

  TreeTarget(String part, String external) {
    this.part = part;
    this.external = external;
  }

  /**
   * The part's name.
   *
   * @return the part's name, or null for an external target
   */
  public String part() {
    return part;
  }

  /**
   * The external target.
   *
   * @return the external target, or null for a part
   */
  public String external() {
    return external;
  }
}
