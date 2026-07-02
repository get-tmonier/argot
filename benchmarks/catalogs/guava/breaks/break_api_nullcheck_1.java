/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.base;

import com.google.common.annotations.GwtCompatible;

/** Argument-validation helpers for splitter configuration. */
@GwtCompatible
final class SeparatorChecks {
  private SeparatorChecks() {}

  /** Returns a splitter-ready trimming matcher description. */
  static String describeTrimmer(CharMatcher trimmer) {
    return "Splitter.trimResults(" + trimmer + ")";
  }

  // Break: hand-rolled null/argument checks throwing raw NullPointerException and
  // concatenated IllegalArgumentException; duplicates the repo's own
  // Preconditions.checkNotNull/checkArgument utility used at 1300+ call sites.
  private static String requireSeparator(String separator) {
    if (separator == null) {
      throw new NullPointerException("separator must not be null");
    }
    if (separator.isEmpty()) {
      throw new IllegalArgumentException("separator may not be empty");
    }
    return separator;
  }

  private static int requireLimit(int limit) {
    if (limit <= 0) {
      throw new IllegalArgumentException("limit must be positive but was " + limit);
    }
    return limit;
  }

  private static Object requireNonNullEntry(Object entry, int index) {
    if (entry == null) {
      throw new NullPointerException("null element at index " + index);
    }
    return entry;
  }
}
