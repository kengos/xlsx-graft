// Reads the keys of a view's JSON after "ok", "error" and "diagnostics" into a Tree or a Grid. A
// key it does not know is skipped. A key that is missing or of the wrong type is a
// Json.UnreadableException, which ViewReport turns into a diagnostics_unreadable failure; nothing
// here checks a value beyond what the static type of a field needs.

package io.github.kengos.xlsxgraft;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;

final class ViewContent {

  private ViewContent() {}

  /** The Tree that the {@code tree} view holds. */
  static Tree tree(Map<String, Object> view) throws Json.UnreadableException {
    List<TreePart> parts = new ArrayList<>();
    for (Object part : list(view, "parts")) {
      parts.add(part(EngineDiagnostics.object(part, "a part")));
    }
    return new Tree(
        flag(view, "complete"),
        flag(view, "relationshipsKnown"),
        relationships(view),
        Collections.unmodifiableList(parts));
  }

  /** The Grid that the {@code describe_sheet} view holds. */
  static Grid grid(Map<String, Object> view) throws Json.UnreadableException {
    Map<String, Object> truncated =
        EngineDiagnostics.object(EngineDiagnostics.member(view, "truncated"), "truncated");
    List<GridRow> rows = new ArrayList<>();
    for (Object row : list(view, "rows")) {
      rows.add(row(EngineDiagnostics.object(row, "a row")));
    }
    List<String> merges = new ArrayList<>();
    for (Object merge : list(view, "merges")) {
      merges.add(EngineDiagnostics.of(String.class, merge, "merges"));
    }
    return new Grid(
        sheet(EngineDiagnostics.member(view, "sheet")),
        nullableText(view, "dimension"),
        nullableText(view, "used"),
        nullableText(view, "shown"),
        new GridTruncated(
            flag(truncated, "rows"), flag(truncated, "columns"), flag(truncated, "merges")),
        flag(view, "complete"),
        Collections.unmodifiableList(rows),
        Collections.unmodifiableList(merges));
  }

  private static TreePart part(Map<String, Object> part) throws Json.UnreadableException {
    Object referenced = EngineDiagnostics.member(part, "referenced");
    return new TreePart(
        EngineDiagnostics.text(part, "name"),
        nullableText(part, "contentType"),
        flag(part, "contentTypeKnown"),
        EngineDiagnostics.of(Long.class, EngineDiagnostics.member(part, "size"), "size"),
        EngineDiagnostics.of(
            Long.class, EngineDiagnostics.member(part, "compressedSize"), "compressedSize"),
        flag(part, "readable"),
        referenced == null ? null : EngineDiagnostics.of(Boolean.class, referenced, "referenced"),
        flag(part, "relationshipsKnown"),
        relationships(part));
  }

  private static List<TreeRelationship> relationships(Map<String, Object> owner)
      throws Json.UnreadableException {
    List<TreeRelationship> relationships = new ArrayList<>();
    for (Object record : list(owner, "relationships")) {
      Map<String, Object> relationship = EngineDiagnostics.object(record, "a relationship");
      relationships.add(
          new TreeRelationship(
              EngineDiagnostics.text(relationship, "id"),
              EngineDiagnostics.text(relationship, "type"),
              target(
                  EngineDiagnostics.object(
                      EngineDiagnostics.member(relationship, "target"), "target"))));
    }
    return Collections.unmodifiableList(relationships);
  }

  // "part" or "external" is absent when the target is not that kind.
  private static TreeTarget target(Map<String, Object> target) throws Json.UnreadableException {
    return new TreeTarget(
        target.containsKey("part") ? nullableText(target, "part") : null,
        target.containsKey("external") ? nullableText(target, "external") : null);
  }

  private static SheetInfo sheet(Object value) throws Json.UnreadableException {
    if (value == null) {
      return null;
    }
    Map<String, Object> sheet = EngineDiagnostics.object(value, "sheet");
    return new SheetInfo(
        integer(sheet, "index"),
        EngineDiagnostics.text(sheet, "name"),
        SheetKind.named(EngineDiagnostics.text(sheet, "kind")),
        SheetState.named(EngineDiagnostics.text(sheet, "state")));
  }

  private static GridRow row(Map<String, Object> row) throws Json.UnreadableException {
    List<GridCell> cells = new ArrayList<>();
    for (Object record : list(row, "cells")) {
      Map<String, Object> cell = EngineDiagnostics.object(record, "a cell");
      cells.add(
          new GridCell(
              EngineDiagnostics.text(cell, "ref"),
              EngineDiagnostics.text(cell, "type"),
              // The value as Json read it: a String, a Boolean, a Long (a number written as an
              // integer) or a Double.
              EngineDiagnostics.member(cell, "value"),
              cell.containsKey("truncated") && flag(cell, "truncated")));
    }
    return new GridRow(integer(row, "row"), Collections.unmodifiableList(cells));
  }

  private static List<?> list(Map<String, Object> owner, String name)
      throws Json.UnreadableException {
    return EngineDiagnostics.of(List.class, EngineDiagnostics.member(owner, name), name);
  }

  private static boolean flag(Map<String, Object> owner, String name)
      throws Json.UnreadableException {
    return EngineDiagnostics.of(Boolean.class, EngineDiagnostics.member(owner, name), name);
  }

  private static String nullableText(Map<String, Object> owner, String name)
      throws Json.UnreadableException {
    Object value = EngineDiagnostics.member(owner, name);
    return value == null ? null : EngineDiagnostics.of(String.class, value, name);
  }

  // A whole number that an int holds.
  private static int integer(Map<String, Object> owner, String name)
      throws Json.UnreadableException {
    long number = EngineDiagnostics.of(Long.class, EngineDiagnostics.member(owner, name), name);
    if (number < Integer.MIN_VALUE || number > Integer.MAX_VALUE) {
      throw new Json.UnreadableException("`" + name + "` is " + number + ", beyond an int");
    }
    return (int) number;
  }
}
