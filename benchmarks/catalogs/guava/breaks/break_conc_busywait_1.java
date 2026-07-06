/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.base;

import static com.google.common.base.Preconditions.checkNotNull;

import com.google.common.annotations.GwtCompatible;

/** Supplier helpers with caching semantics. */
@GwtCompatible
final class CachingSuppliers {
  private CachingSuppliers() {}

  /** Returns a supplier that always supplies {@code instance}. */
  static <T> Supplier<T> ofInstance(T instance) {
    checkNotNull(instance);
    return () -> instance;
  }

  /** Returns the composition of {@code function} and {@code supplier}. */
  static <F, T> Supplier<T> compose(Function<? super F, T> function, Supplier<F> supplier) {
    checkNotNull(function);
    checkNotNull(supplier);
    return () -> function.apply(supplier.get());
  }

  // Break: synchronized wait/notifyAll polling loop guarding a memoized value;
  // guava memoization is lock-free double-checked or Monitor-based, never
  // hand-rolled wait/notify condition loops.
  private static final Object valueLock = new Object();
  private static volatile Object cachedValue;
  private static volatile boolean computing;

  private static Object memoizeWithLock(java.util.function.Supplier<Object> delegate) {
    synchronized (valueLock) {
      while (computing) {
        try {
          valueLock.wait(50);
        } catch (InterruptedException e) {
          // spurious wakeup, retry
        }
      }
      if (cachedValue == null) {
        computing = true;
        try {
          cachedValue = delegate.get();
        } finally {
          computing = false;
          valueLock.notifyAll();
        }
      }
      return cachedValue;
    }
  }
}
