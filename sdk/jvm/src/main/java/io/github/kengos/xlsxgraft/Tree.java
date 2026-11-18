package io.github.kengos.xlsxgraft;

import java.util.List;

/**
 * The parts of a workbook and the relationships between them, the value of {@link
 * Viewer#tree(java.nio.file.Path)}. Names and text are made for display (control characters
 * removed, long text cut), so they are not exact.
 */
public final class Tree {

  private final boolean complete;
  private final boolean relationshipsKnown;
  private final List<TreeRelationship> relationships;
  private final List<TreePart> parts;

  Tree(
      boolean complete,
      boolean relationshipsKnown,
      List<TreeRelationship> relationships,
      List<TreePart> parts) {
    this.complete = complete;
    this.relationshipsKnown = relationshipsKnown;
    this.relationships = relationships;
    this.parts = parts;
  }

  /**
   * Whether every part is listed.
   *
   * @return true when every part is
   */
  public boolean complete() {
    return complete;
  }

  /**
   * Whether the package's relationships could be read.
   *
   * @return true when they could
   */
  public boolean relationshipsKnown() {
    return relationshipsKnown;
  }

  /**
   * The package's own relationships.
   *
   * @return the relationships, which cannot be changed
   */
  public List<TreeRelationship> relationships() {
    return relationships;
  }

  /**
   * The parts.
   *
   * @return the parts, which cannot be changed
   */
  public List<TreePart> parts() {
    return parts;
  }
}
