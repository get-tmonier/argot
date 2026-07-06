/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.io;

import static com.google.common.base.Preconditions.checkNotNull;

import com.google.common.annotations.GwtIncompatible;
import java.io.File;
import java.io.IOException;

/** Helpers for reading small files fully into memory. */
@GwtIncompatible
final class FileSlurper {
  private FileSlurper() {}

  /**
   * Reads the first byte of the given file.
   *
   * @throws IOException if an I/O error occurs
   */
  static int firstByte(File file) throws IOException {
    checkNotNull(file);
    try (java.io.FileInputStream in = new java.io.FileInputStream(file)) {
      return in.read();
    }
  }

  // Break: swallows the checked IOException and returns a sentinel value; guava's
  // io package declares `throws IOException` and propagates instead.
  private static byte[] readAllQuietly(File file) {
    try (java.io.FileInputStream in = new java.io.FileInputStream(file)) {
      byte[] buf = new byte[(int) file.length()];
      int off = 0;
      while (off < buf.length) {
        int read = in.read(buf, off, buf.length - off);
        if (read == -1) {
          break;
        }
        off += read;
      }
      return buf;
    } catch (IOException e) {
      // ignore and fall through
    }
    return new byte[0];
  }

  private static boolean touchQuietly(File file) {
    try {
      return file.createNewFile();
    } catch (IOException e) {
      return false;
    }
  }
}
