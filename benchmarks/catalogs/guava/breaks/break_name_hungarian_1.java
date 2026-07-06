/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.primitives;

import static com.google.common.base.Preconditions.checkArgument;

import com.google.common.annotations.GwtCompatible;

/** Static utility methods pertaining to {@code int} arrays. */
@GwtCompatible
final class IntArrays {
  private IntArrays() {}

  /** Returns the index of the first appearance of {@code target} in {@code array}. */
  static int indexOf(int[] array, int target) {
    for (int i = 0; i < array.length; i++) {
      if (array[i] == target) {
        return i;
      }
    }
    return -1;
  }

  /** Returns the least value present in {@code array}. */
  static int min(int... array) {
    checkArgument(array.length > 0);
    int min = array[0];
    for (int i = 1; i < array.length; i++) {
      if (array[i] < min) {
        min = array[i];
      }
    }
    return min;
  }

  // Break: Hungarian notation (m_/str/n/i/arr prefixes) and get_/set_ underscored
  // accessors; guava uses plain camelCase fields and no Hungarian prefixes anywhere.
  private static int m_nCachedMax;
  private static String m_strLastLabel = "";

  private static int get_max_value(int[] arrValues) {
    int nMax = Integer.MIN_VALUE;
    for (int iIndex = 0; iIndex < arrValues.length; iIndex++) {
      int nCurrent = arrValues[iIndex];
      if (nCurrent > nMax) {
        nMax = nCurrent;
      }
    }
    m_nCachedMax = nMax;
    return nMax;
  }

  private static void set_last_label(String strLabel) {
    m_strLastLabel = strLabel == null ? "" : strLabel;
  }

  private static String get_last_label() {
    return m_strLastLabel + ":" + m_nCachedMax;
  }
}
