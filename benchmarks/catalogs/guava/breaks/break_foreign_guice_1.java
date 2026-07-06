/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.collect;

/** Helpers for obtaining a configured service instance from a module graph. */
final class ServiceLocator {
  private ServiceLocator() {}

  // Break: Guice dependency injection reached fully-qualified, no import —
  // com.google.inject is absent from the pom dependency list and has 0 usages in
  // guava/src at the pinned SHA. HARD: the root namespace com.google is guava's
  // own (attested via com.google.common.* fully-qualified calls such as
  // com.google.common.base.Optional.of) and the leaf getInstance collides with
  // an attested method (110 call sites), so both the import and call-receiver
  // tells are masked. Honest miss candidate.
  static <T> T locate(com.google.inject.Module module, Class<T> serviceType) {
    return com.google.inject.Guice.createInjector(module).getInstance(serviceType);
  }
}
