// Workbooks for the tests to read, built as a test asks: a zip holding a
// package with a shared strings table of "a" and "b", styles whose cell
// formats 1 to 4 are a date, a time, a date-time and an elapsed time, and the
// sheets the test gives. The same workbook as engine/capi/src/tests/workbook.rs
// builds, written again in Java.

package io.github.kengos.xlsxgraft;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

final class Workbook {

  private static final String NS = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
  private static final String TYPES =
      "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
  private static final String RELS = "http://schemas.openxmlformats.org/package/2006/relationships";
  private static final String CONTENT_TYPES =
      "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">"
          + "<Default Extension=\"rels\""
          + " ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>"
          + "<Default Extension=\"xml\" ContentType=\"application/xml\"/></Types>";

  // The number formats of cell formats 0 to 4: general, a date, a time, a
  // date-time, an elapsed time.
  private static final int[] FORMATS = {0, 14, 21, 22, 46};

  private Workbook() {}

  /** A worksheet named {@code name} whose sheetData holds {@code rows}. */
  static Sheet worksheet(String name, String rows) {
    return worksheet(name, rows, null);
  }

  /** As {@link #worksheet(String, String)}, of the state given (null: none written). */
  static Sheet worksheet(String name, String rows, String state) {
    return new Sheet(
        name,
        state,
        TYPES + "/worksheet",
        "<worksheet xmlns=\"" + NS + "\"><sheetData>" + rows + "</sheetData></worksheet>");
  }

  static Sheet chartsheet(String name) {
    return new Sheet(name, null, TYPES + "/chartsheet", "<chartsheet xmlns=\"" + NS + "\"/>");
  }

  /** A sheet named {@code name} whose r:id names no relationship. */
  static Sheet unrelated(String name) {
    return new Sheet(name, null, null, "");
  }

  /** The bytes of a workbook of {@code sheets}, in order, each at xl/sheets/sheet<n>.xml. */
  static byte[] xlsx(Sheet... sheets) {
    StringBuilder listed = new StringBuilder();
    StringBuilder related = new StringBuilder();
    Map<String, String> parts = new LinkedHashMap<>();
    for (int i = 0; i < sheets.length; i++) {
      Sheet sheet = sheets[i];
      int n = i + 1;
      listed
          .append("<sheet name=\"")
          .append(sheet.name)
          .append("\" sheetId=\"")
          .append(n)
          .append('"')
          .append(sheet.state == null ? "" : " state=\"" + sheet.state + "\"")
          .append(" r:id=\"rId")
          .append(n)
          .append("\"/>");
      if (sheet.relType != null) {
        related.append(rel("rId" + n, sheet.relType, "sheets/sheet" + n + ".xml"));
        parts.put("xl/sheets/sheet" + n + ".xml", sheet.xml);
      }
    }
    StringBuilder formats = new StringBuilder();
    for (int id : FORMATS) {
      formats.append("<xf numFmtId=\"").append(id).append("\"/>");
    }

    Map<String, String> entries = new LinkedHashMap<>();
    entries.put("[Content_Types].xml", CONTENT_TYPES);
    entries.put(
        "_rels/.rels", relationships(rel("rId1", TYPES + "/officeDocument", "xl/workbook.xml")));
    entries.put(
        "xl/workbook.xml",
        "<workbook xmlns=\""
            + NS
            + "\" xmlns:r=\""
            + TYPES
            + "\"><sheets>"
            + listed
            + "</sheets></workbook>");
    entries.put(
        "xl/_rels/workbook.xml.rels",
        relationships(
            rel("rIdS", TYPES + "/sharedStrings", "sst.xml")
                + rel("rIdT", TYPES + "/styles", "styles.xml")
                + related));
    entries.put("xl/sst.xml", "<sst xmlns=\"" + NS + "\"><si><t>a</t></si><si><t>b</t></si></sst>");
    entries.put(
        "xl/styles.xml",
        "<styleSheet xmlns=\"" + NS + "\"><cellXfs>" + formats + "</cellXfs></styleSheet>");
    entries.putAll(parts);
    return zip(entries);
  }

  private static String rel(String id, String type, String target) {
    return "<Relationship Id=\"" + id + "\" Type=\"" + type + "\" Target=\"" + target + "\"/>";
  }

  private static String relationships(String body) {
    return "<Relationships xmlns=\"" + RELS + "\">" + body + "</Relationships>";
  }

  private static byte[] zip(Map<String, String> entries) {
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    try (ZipOutputStream zip = new ZipOutputStream(bytes)) {
      for (Map.Entry<String, String> entry : entries.entrySet()) {
        zip.putNextEntry(new ZipEntry(entry.getKey()));
        zip.write(entry.getValue().getBytes(StandardCharsets.UTF_8));
        zip.closeEntry();
      }
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
    return bytes.toByteArray();
  }

  /**
   * A sheet: its name, its state (null: none written), the type of its relationship (null: its r:id
   * names none) and its part's XML.
   */
  static final class Sheet {

    final String name;
    final String state;
    final String relType;
    final String xml;

    Sheet(String name, String state, String relType, String xml) {
      this.name = name;
      this.state = state;
      this.relType = relType;
      this.xml = xml;
    }
  }
}
