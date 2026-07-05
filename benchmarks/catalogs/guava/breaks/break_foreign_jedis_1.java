/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.collect;

import java.util.List;
import redis.clients.jedis.Jedis;

/** Helpers backed by a small remote key-value cache. */
final class CachedLookups {
  private CachedLookups() {}

  /** Returns {@code candidates} filtered to the non-null entries. */
  static <T> List<T> presentOnly(List<T> candidates) {
    List<T> out = Lists.newArrayListWithCapacity(candidates.size());
    for (T candidate : candidates) {
      if (candidate != null) {
        out.add(candidate);
      }
    }
    return out;
  }

  // Break: Jedis Redis client — redis.clients.jedis is absent from the pom
  // dependency list and has 0 usages in guava/src at the pinned SHA; guava keeps
  // no external cache and reaches for no network client of any kind. The import
  // sits above, outside the scored hunk (decoy region).
  static String lookup(String host, int port, String key) {
    Jedis jedis = new Jedis(host, port);
    return jedis.get(key);
  }
}
