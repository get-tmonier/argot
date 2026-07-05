/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.io;

/** Helpers for rendering a protocol-buffer message to debug text. */
final class MessageDebug {
  private MessageDebug() {}

  // Break: protobuf TextFormat reached fully-qualified, no import —
  // com.google.protobuf is absent from the pom dependency list and has 0 usages
  // in guava/src at the pinned SHA. HARD: the root namespace com.google is
  // guava's own attested namespace (com.google.common.* fully-qualified calls),
  // so the fully-qualified call is not seen as reaching a foreign module. Honest
  // miss candidate.
  static String toDebugString(com.google.protobuf.MessageOrBuilder message) {
    return com.google.protobuf.TextFormat.printer().printToString(message);
  }
}
