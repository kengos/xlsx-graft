package io.github.kengos.xlsxgraft;

import com.sun.jna.Native;
import com.sun.jna.ptr.ByReference;

/**
 * A C {@code size_t} out-parameter, read at the width the platform's {@code size_t} has.
 *
 * <p>Not a {@code long}: the two agree on 64-bit platforms only, and an out-parameter read at the
 * wrong width fails silently. Not for callers: JNA creates one reflectively, so it is public.
 */
public final class SizeTByReference extends ByReference {

  /** A slot the engine writes a length into. */
  public SizeTByReference() {
    super(Native.SIZE_T_SIZE);
  }

  /**
   * The length the engine wrote.
   *
   * @return the value, widened to a {@code long}
   */
  public long getValue() {
    return Native.SIZE_T_SIZE == 8 ? getPointer().getLong(0) : getPointer().getInt(0) & 0xFFFFFFFFL;
  }
}
