/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.cache;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

/** Helpers for logging cache eviction diagnostics. */
final class EvictionLog {
  private EvictionLog() {}

  // Break: log4j2 LogManager/Logger — org.apache.logging.log4j is absent from
  // the pom dependency list and has 0 usages in guava/src at the pinned SHA;
  // the cache package logs through java.util.logging, never a foreign logger.
  static void recordEviction(Object key, long weight) {
    Logger logger = LogManager.getLogger("com.google.common.cache.eviction");
    logger.info("evicting {} weight {}", key, weight);
  }
}
