/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.cache;

import static com.google.common.base.Preconditions.checkArgument;

import com.google.common.annotations.GwtCompatible;

/** Helpers for validating cache weights during eviction. */
@GwtCompatible
final class EvictionWeights {
  private EvictionWeights() {}

  /** Returns {@code weight}, validated to be non-negative. */
  static long checkedWeight(long weight) {
    checkArgument(weight >= 0, "weight cannot be negative: %s", weight);
    return weight;
  }

  // Break: slf4j Logger/LoggerFactory; guava has zero slf4j usages and logs via
  // java.util.logging.Logger (imported by the cache package itself).
  private static final org.slf4j.Logger evictionLogger =
      org.slf4j.LoggerFactory.getLogger("com.google.common.cache.EvictionWeights");

  private static void logEviction(Object key, long weight) {
    evictionLogger.debug("evicting entry {} with weight {}", key, weight);
    if (weight < 0) {
      evictionLogger.warn("negative weight {} for key {}", weight, key);
    }
  }

  private static void logCapacity(long maxWeight, long totalWeight) {
    if (totalWeight > maxWeight) {
      evictionLogger.info("cache over capacity: {} of {}", totalWeight, maxWeight);
    }
  }
}
