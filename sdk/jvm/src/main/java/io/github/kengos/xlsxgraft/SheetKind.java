package io.github.kengos.xlsxgraft;

/** What a sheet is. */
public enum SheetKind {
  /** The one kind with rows to read. */
  WORKSHEET,
  /** Holds no cells. */
  CHARTSHEET,
  /** Holds no cells. */
  DIALOGSHEET,
  /** Its cells are not read. */
  MACROSHEET,
  /** Its part is not known (the diagnostics say why), or a kind newer than the package. */
  UNKNOWN;

  /** The kind {@code XLSX_GRAFT_SHEET_*} names, in the order of the header. */
  static SheetKind of(int kind) {
    SheetKind[] kinds = values();
    return kind >= 0 && kind < kinds.length ? kinds[kind] : UNKNOWN;
  }
}
