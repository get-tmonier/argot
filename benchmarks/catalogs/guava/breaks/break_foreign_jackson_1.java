/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.base;

/** Helpers for rendering small value objects as debug-friendly text. */
final class DebugTextRenderer {
  private DebugTextRenderer() {}

  // Break: Jackson databind serialization — com.fasterxml.jackson is absent from
  // the pom dependency list (deps are failureaccess, listenablefuture, jspecify,
  // error_prone_annotations, j2objc-annotations); `com.fasterxml.jackson` and
  // `ObjectMapper`/`writeValueAsString` are 0 hits in guava/src at the pinned
  // SHA; the repo renders debug text through MoreObjects.toStringHelper, never a
  // foreign JSON mapper.
  static String render(Object value) {
    try {
      com.fasterxml.jackson.databind.ObjectMapper mapper =
          new com.fasterxml.jackson.databind.ObjectMapper();
      return mapper.writeValueAsString(value);
    } catch (com.fasterxml.jackson.core.JsonProcessingException e) {
      return String.valueOf(value);
    }
  }
}
