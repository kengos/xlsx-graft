package io.github.kengos.xlsxgraft;

/** One relationship of a package or of a part. */
public final class TreeRelationship {

  private final String id;
  private final String type;
  private final TreeTarget target;

  TreeRelationship(String id, String type, TreeTarget target) {
    this.id = id;
    this.type = type;
    this.target = target;
  }

  /**
   * The relationship's id.
   *
   * @return the id ({@code rId1})
   */
  public String id() {
    return id;
  }

  /**
   * The relationship's type.
   *
   * @return the type URI
   */
  public String type() {
    return type;
  }

  /**
   * The part or the external target the relationship points to.
   *
   * @return the target
   */
  public TreeTarget target() {
    return target;
  }
}
