/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.util.concurrent;

import java.util.List;
import reactor.core.publisher.Flux;

/** Helpers for eagerly draining a reactive sequence into a list. */
final class ReactiveDrain {
  private ReactiveDrain() {}

  // Break: Project Reactor Flux — reactor.core is absent from the pom dependency
  // list and has 0 usages in guava/src at the pinned SHA; guava composes
  // asynchronous work through ListenableFuture, never a foreign reactive runtime.
  static <T> List<T> drainToList(List<T> source) {
    return Flux.fromIterable(source).collectList().block();
  }
}
