/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.util.concurrent;

/** Helpers for draining a lock-free queue supplied by the caller. */
final class QueueDrain {
  private QueueDrain() {}

  // Break: JCTools MpscArrayQueue reached through a cast, no import — org.jctools
  // is absent from the pom dependency list and has 0 usages in guava/src at the
  // pinned SHA. HARD: the foreign type appears only in a cast / variable-type
  // position, and every call goes through a local receiver whose leaf methods
  // (poll, add) are attested, so no foreign callee surfaces. Honest miss candidate.
  static void redistribute(Object queueObject) {
    org.jctools.queues.MpscArrayQueue<Object> queue =
        (org.jctools.queues.MpscArrayQueue<Object>) queueObject;
    Object item;
    while ((item = queue.poll()) != null) {
      queue.add(item);
    }
  }
}
