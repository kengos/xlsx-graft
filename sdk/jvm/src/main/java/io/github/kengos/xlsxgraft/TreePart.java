package io.github.kengos.xlsxgraft;

import java.util.List;

/** One part of a package, as the engine's {@code tree} shows it. */
public final class TreePart {

  private final String name;
  private final String contentType;
  private final boolean contentTypeKnown;
  private final long size;
  private final long compressedSize;
  private final boolean readable;
  private final Boolean referenced;
  private final boolean relationshipsKnown;
  private final List<TreeRelationship> relationships;

  TreePart(
      String name,
      String contentType,
      boolean contentTypeKnown,
      long size,
      long compressedSize,
      boolean readable,
      Boolean referenced,
      boolean relationshipsKnown,
      List<TreeRelationship> relationships) {
    this.name = name;
    this.contentType = contentType;
    this.contentTypeKnown = contentTypeKnown;
    this.size = size;
    this.compressedSize = compressedSize;
    this.readable = readable;
    this.referenced = referenced;
    this.relationshipsKnown = relationshipsKnown;
    this.relationships = relationships;
  }

  /**
   * The part's name.
   *
   * @return the name ({@code /xl/workbook.xml})
   */
  public String name() {
    return name;
  }

  /**
   * The part's content type.
   *
   * @return the content type, or null when none is declared
   */
  public String contentType() {
    return contentType;
  }

  /**
   * Whether the content type could be determined.
   *
   * @return true when the content type could be determined
   */
  public boolean contentTypeKnown() {
    return contentTypeKnown;
  }

  /**
   * The part's size in bytes, uncompressed.
   *
   * @return the size
   */
  public long size() {
    return size;
  }

  /**
   * The part's size in the archive.
   *
   * @return the size
   */
  public long compressedSize() {
    return compressedSize;
  }

  /**
   * Whether the part could be read.
   *
   * @return true when the part could be read
   */
  public boolean readable() {
    return readable;
  }

  /**
   * Whether a relationship points at the part.
   *
   * @return true or false, or null when that is not known
   */
  public Boolean referenced() {
    return referenced;
  }

  /**
   * Whether the part's relationships could be read.
   *
   * @return true when they could
   */
  public boolean relationshipsKnown() {
    return relationshipsKnown;
  }

  /**
   * The relationships the part holds.
   *
   * @return the relationships, which cannot be changed
   */
  public List<TreeRelationship> relationships() {
    return relationships;
  }
}
