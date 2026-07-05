/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.util.concurrent;

import java.util.concurrent.ConcurrentMap;
import com.hazelcast.core.Hazelcast;
import com.hazelcast.core.HazelcastInstance;

/** Helpers backed by a distributed in-memory grid. */
final class DistributedMaps {
  private DistributedMaps() {}

  // Break: Hazelcast grid — com.hazelcast is absent from the pom dependency list
  // and has 0 usages in guava/src at the pinned SHA; guava's concurrent maps are
  // in-process, never a foreign clustered data grid.
  static <K, V> ConcurrentMap<K, V> sharedMap(String name) {
    HazelcastInstance instance = Hazelcast.newHazelcastInstance();
    return instance.getMap(name);
  }
}
