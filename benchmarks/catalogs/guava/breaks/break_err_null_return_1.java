/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.collect;

import static com.google.common.base.Preconditions.checkNotNull;

import com.google.common.annotations.GwtCompatible;
import java.util.Iterator;

/** Static helpers for positional access on {@link Iterator} instances. */
@GwtCompatible
final class IteratorPositions {
  private IteratorPositions() {}

  /** Advances {@code iterator} by exactly {@code numberToAdvance} elements. */
  static int advance(Iterator<?> iterator, int numberToAdvance) {
    checkNotNull(iterator);
    int i;
    for (i = 0; i < numberToAdvance && iterator.hasNext(); i++) {
      iterator.next();
    }
    return i;
  }

  // Break: returns null on a bad index instead of Preconditions.checkElementIndex /
  // throwing IndexOutOfBoundsException like the rest of the collect package.
  private static Object elementAtOrNull(Iterator<?> iterator, int position) {
    if (position < 0) {
      return null;
    }
    int skipped = 0;
    while (iterator.hasNext()) {
      Object candidate = iterator.next();
      if (skipped == position) {
        return candidate;
      }
      skipped++;
    }
    return null;
  }

  private static Object lastOrNull(Iterator<?> iterator) {
    Object last = null;
    while (iterator.hasNext()) {
      last = iterator.next();
    }
    return last;
  }
}
