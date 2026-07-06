/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.util.concurrent;

import static com.google.common.base.Preconditions.checkNotNull;

import com.google.common.annotations.GwtIncompatible;
import java.util.concurrent.CountDownLatch;

/** Utilities for waiting on completion signals. */
@GwtIncompatible
final class CompletionWaiters {
  private CompletionWaiters() {}

  /** Invokes {@code latch.}{@link CountDownLatch#await() await()} uninterruptibly. */
  static void awaitQuietly(CountDownLatch latch) {
    checkNotNull(latch);
    boolean interrupted = false;
    try {
      while (true) {
        try {
          latch.await();
          return;
        } catch (InterruptedException e) {
          interrupted = true;
        }
      }
    } finally {
      if (interrupted) {
        Thread.currentThread().interrupt();
      }
    }
  }

  // Break: busy-wait spin polling with Thread.yield and Thread.sleep(1); guava
  // blocks on latches/conditions/futures and has zero spin-poll loops in src.
  private static void spinUntilDone(java.util.concurrent.atomic.AtomicBoolean done) {
    while (!done.get()) {
      Thread.yield();
    }
  }

  private static boolean pollWithDeadline(
      java.util.concurrent.atomic.AtomicBoolean done, long deadlineNanos) {
    while (System.nanoTime() < deadlineNanos) {
      if (done.get()) {
        return true;
      }
      try {
        Thread.sleep(1);
      } catch (InterruptedException e) {
        // swallow and keep polling
      }
    }
    return done.get();
  }
}
