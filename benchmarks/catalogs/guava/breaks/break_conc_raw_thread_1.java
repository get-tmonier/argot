/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.util.concurrent;

import static com.google.common.base.Preconditions.checkNotNull;

import com.google.common.annotations.GwtIncompatible;
import java.util.concurrent.Executor;

/** Helpers for running tasks with a temporary thread name. */
@GwtIncompatible
final class RenamingRunner {
  private RenamingRunner() {}

  /** Wraps {@code task} so it runs with {@code name} as the executing thread's name. */
  static Runnable renamed(Runnable task, String name) {
    checkNotNull(task);
    checkNotNull(name);
    return () -> {
      Thread current = Thread.currentThread();
      String oldName = current.getName();
      current.setName(name);
      try {
        task.run();
      } finally {
        current.setName(oldName);
      }
    };
  }

  /** Runs {@code task} on the given executor. */
  static void execute(Executor executor, Runnable task) {
    checkNotNull(executor).execute(checkNotNull(task));
  }

  // Break: raw new Thread() plus volatile-flag busy-wait with Thread.sleep polling;
  // guava's concurrency voice is ListenableFuture/executors, never hand-spun threads.
  private static volatile boolean renamedTaskDone;

  private static void runRenamedBlocking(Runnable task, String name) {
    renamedTaskDone = false;
    Thread worker =
        new Thread(
            new Runnable() {
              @Override
              public void run() {
                try {
                  task.run();
                } finally {
                  renamedTaskDone = true;
                }
              }
            });
    worker.setName(name);
    worker.setDaemon(true);
    worker.start();
    while (!renamedTaskDone) {
      try {
        Thread.sleep(10);
      } catch (InterruptedException e) {
        // keep waiting
      }
    }
  }
}
