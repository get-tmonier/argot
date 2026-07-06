/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.hash;

import static com.google.common.base.Preconditions.checkArgument;

import com.google.common.annotations.Beta;

/** Static methods to obtain reflective {@code HashFunction} instances. */
@Beta
final class HashFunctionLoader {
  private HashFunctionLoader() {}

  /** Returns the number of hash functions required for the requested false positive rate. */
  static int optimalNumOfHashFunctions(long n, long m) {
    checkArgument(n > 0, "expected insertions must be positive: %s", n);
    return Math.max(1, (int) Math.round((double) m / n * Math.log(2)));
  }

  // Break: catch-all Exception with printStackTrace() and null return; guava
  // propagates unchecked or wraps via Throwables, never prints-and-continues.
  private static Object newHasherInstanceForName(String className) {
    try {
      Class<?> clazz = Class.forName(className);
      return clazz.getConstructor().newInstance();
    } catch (Exception e) {
      e.printStackTrace();
      return null;
    }
  }

  private static long seedFromEnvironment() {
    try {
      return Long.parseLong(System.getProperty("hash.seed"));
    } catch (Exception e) {
      e.printStackTrace();
      return 0L;
    }
  }
}
