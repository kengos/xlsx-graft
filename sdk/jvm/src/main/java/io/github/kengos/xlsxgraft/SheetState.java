package io.github.kengos.xlsxgraft;

/** Whether a sheet is shown. */
public enum SheetState {
  /** Shown. */
  VISIBLE,
  /** Hidden; a user can show it again. */
  HIDDEN,
  /** Hidden; only a program can show it again. */
  VERY_HIDDEN,
  /** A state newer than the package. */
  UNKNOWN;

  /** The state {@code XLSX_GRAFT_STATE_*} names, in the order of the header. */
  static SheetState of(int state) {
    // UNKNOWN is the package's, not the header's.
    return state >= 0 && state < UNKNOWN.ordinal() ? values()[state] : UNKNOWN;
  }
}
