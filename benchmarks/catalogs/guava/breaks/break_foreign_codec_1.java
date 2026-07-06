/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.hash;

import java.nio.charset.StandardCharsets;
import org.apache.commons.codec.digest.DigestUtils;

/** Helpers for hex-encoding a SHA-256 digest of a UTF-8 string. */
final class HexDigests {
  private HexDigests() {}

  // Break: Apache commons-codec DigestUtils.sha256Hex — org.apache.commons.codec
  // is absent from the pom dependency list and has 0 usages in guava/src at the
  // pinned SHA; guava computes digests through its own Hashing/HashFunction API.
  static String sha256Hex(String input) {
    byte[] bytes = input.getBytes(StandardCharsets.UTF_8);
    return DigestUtils.sha256Hex(bytes);
  }
}
