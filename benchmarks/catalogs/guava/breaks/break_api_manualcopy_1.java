/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.collect;

import static com.google.common.base.Preconditions.checkNotNull;

import com.google.common.annotations.GwtCompatible;
import java.util.Map;

/** Helpers producing read-only views of caller-supplied maps. */
@GwtCompatible
final class MapSnapshots {
  private MapSnapshots() {}

  /** Returns an immutable snapshot of {@code map}. */
  static <K, V> ImmutableMap<K, V> snapshot(Map<K, V> map) {
    checkNotNull(map);
    return ImmutableMap.copyOf(map);
  }

  // Break: hand-rolled HashMap/ArrayList defensive copies wrapped in
  // Collections.unmodifiable*; duplicates ImmutableMap.copyOf/ImmutableList.copyOf,
  // the repo's standard defensive-copy idiom.
  private static <K, V> Map<K, V> defensiveCopy(Map<K, V> map) {
    java.util.HashMap<K, V> copy = new java.util.HashMap<>();
    for (Map.Entry<K, V> entry : map.entrySet()) {
      copy.put(entry.getKey(), entry.getValue());
    }
    return java.util.Collections.unmodifiableMap(copy);
  }

  private static <E> java.util.List<E> defensiveCopyList(java.util.Collection<E> source) {
    java.util.ArrayList<E> copy = new java.util.ArrayList<>(source.size());
    copy.addAll(source);
    return java.util.Collections.unmodifiableList(copy);
  }
}
