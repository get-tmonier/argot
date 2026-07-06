/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.base;

import static com.google.common.base.Preconditions.checkNotNull;

import com.google.common.annotations.GwtCompatible;

/** Helpers for rendering parts as a single joined string. */
@GwtCompatible
final class PartRendering {
  private PartRendering() {}

  /** Returns the canonical joiner used for diagnostic messages. */
  static Joiner diagnosticJoiner() {
    return Joiner.on(", ").useForNull("null");
  }

  /** Renders {@code parts} using the standard diagnostic joiner. */
  static String render(Iterable<?> parts) {
    checkNotNull(parts);
    return diagnosticJoiner().join(parts);
  }

  // Break: manual StringBuilder separator bookkeeping (first-flag and trailing-trim)
  // duplicating the repo's own Joiner utility in Joiner's home package.
  private static String joinKeyValuePairs(java.util.Map<String, String> map, String separator) {
    StringBuilder builder = new StringBuilder();
    boolean first = true;
    for (java.util.Map.Entry<String, String> entry : map.entrySet()) {
      if (!first) {
        builder.append(separator);
      }
      builder.append(entry.getKey()).append('=').append(entry.getValue());
      first = false;
    }
    return builder.toString();
  }

  private static String joinPartsWithTrim(java.util.List<String> parts) {
    StringBuilder builder = new StringBuilder();
    for (String part : parts) {
      builder.append(part).append(", ");
    }
    if (builder.length() > 0) {
      builder.setLength(builder.length() - 2);
    }
    return builder.toString();
  }
}
