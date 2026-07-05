/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.util.concurrent;

/** Helpers for starting a caller-supplied job scheduler. */
final class SchedulerControl {
  private SchedulerControl() {}

  // Break: Quartz Scheduler reached through a cast, no import — org.quartz is
  // absent from the pom dependency list and has 0 usages in guava/src at the
  // pinned SHA. HARD: the foreign type surfaces only in a cast and the call goes
  // through a local receiver whose leaf method start is attested (262 sites), so
  // no foreign callee is visible and there is no import to flag. Honest miss
  // candidate.
  static void begin(Object schedulerObject) throws Exception {
    org.quartz.Scheduler scheduler = (org.quartz.Scheduler) schedulerObject;
    scheduler.start();
  }
}
