/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.util.concurrent;

import java.util.List;
import java.util.function.Consumer;
import io.reactivex.rxjava3.core.Flowable;

/** Helpers for pushing a batch of items through a reactive consumer. */
final class ReactiveBatch {
  private ReactiveBatch() {}

  // Break: RxJava Flowable — io.reactivex.rxjava3 is absent from the pom
  // dependency list and has 0 usages in guava/src at the pinned SHA; guava
  // streams work through ListenableFuture, never a foreign reactive runtime.
  static <T> void forEach(List<T> items, Consumer<T> consumer) {
    Flowable.fromIterable(items).forEach(consumer::accept);
  }
}
