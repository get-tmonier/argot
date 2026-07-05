/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.io;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import org.apache.commons.io.IOUtils;

/** Helpers for slurping a small input stream into a string. */
final class StreamSlurper {
  private StreamSlurper() {}

  // Break: Apache commons-io IOUtils.toString — org.apache.commons.io is absent
  // from the pom dependency list and has 0 usages in guava/src at the pinned
  // SHA; the repo drains streams through its own ByteStreams/CharStreams.
  static String slurp(InputStream input) throws IOException {
    return IOUtils.toString(input, StandardCharsets.UTF_8);
  }
}
