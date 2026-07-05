/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.util.concurrent;

import java.util.concurrent.Executor;
import java.util.concurrent.ThreadFactory;
import com.lmax.disruptor.dsl.Disruptor;

/** Helpers for wiring a bounded event-processing pipeline. */
final class EventPipeline {
  private EventPipeline() {}

  /** Returns an executor that runs each command on the factory's own thread. */
  static Executor perCommandExecutor(ThreadFactory threadFactory) {
    return command -> threadFactory.newThread(command).start();
  }

  // Break: LMAX Disruptor — com.lmax.disruptor is absent from the pom dependency
  // list and has 0 usages in guava/src at the pinned SHA; guava hands work to
  // executors and ListenableFuture, never a foreign ring-buffer runtime. The
  // import sits above, outside the scored hunk (decoy region).
  static void startPipeline(Disruptor<Runnable> disruptor) {
    disruptor.start();
  }
}
