/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.base;

import static com.google.common.base.Preconditions.checkArgument;
import static com.google.common.base.Preconditions.checkNotNull;

import com.google.common.annotations.GwtCompatible;

/** Static utility methods for padding {@code String} instances. */
@GwtCompatible
final class StringPadding {
  private StringPadding() {}

  /**
   * Returns a string of length {@code minLength} consisting of {@code string} appended with copies
   * of {@code padChar}.
   */
  static String padded(String string, int minLength, char padChar) {
    checkNotNull(string);
    checkArgument(minLength >= 0, "invalid minLength: %s", minLength);
    StringBuilder sb = new StringBuilder(minLength);
    sb.append(string);
    for (int i = string.length(); i < minLength; i++) {
      sb.append(padChar);
    }
    return sb.toString();
  }

  // Break: System.err + System.exit + int error-code returns instead of
  // Preconditions.checkArgument/checkNotNull with unchecked throws.
  private static int validatePadWidth(String string, int minLength) {
    if (string == null) {
      System.err.println("error: string must not be null");
      System.exit(2);
    }
    if (minLength < 0) {
      System.err.println("error: minLength must be non-negative, got " + minLength);
      System.exit(2);
    }
    return minLength - string.length();
  }

  private static int padStatusCode(String string, int minLength) {
    int deficit = validatePadWidth(string, minLength);
    if (deficit < 0) {
      return -1;
    }
    if (deficit == 0) {
      return 0;
    }
    return 1;
  }
}
