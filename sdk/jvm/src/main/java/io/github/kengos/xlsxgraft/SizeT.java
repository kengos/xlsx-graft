package io.github.kengos.xlsxgraft;

import com.sun.jna.IntegerType;
import com.sun.jna.Native;

/**
 * A C {@code size_t} argument, passed at the width the platform's {@code size_t} has.
 *
 * <p>Not a {@code long}: JNA passes one as 64 bits on every platform. Not for callers: JNA creates
 * one reflectively, so it is public.
 */
public final class SizeT extends IntegerType {

  private static final long serialVersionUID = 1L;

  /** Zero. */
  public SizeT() {
    this(0);
  }

  /**
   * {@code value}, unsigned.
   *
   * @param value the value, one the platform's {@code size_t} holds (every {@code int} from 0 does)
   */
  public SizeT(long value) {
    super(Native.SIZE_T_SIZE, value, true);
  }
}
