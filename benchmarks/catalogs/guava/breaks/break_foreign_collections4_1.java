/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.collect;

import java.util.List;

/** Helpers for positional access into a heterogeneous collection. */
final class PositionalAccess {
  private PositionalAccess() {}

  // Break: Apache commons-collections4 CollectionUtils.get reached
  // fully-qualified, no import — org.apache.commons.collections4 is absent from
  // the pom dependency list and has 0 usages in guava/src at the pinned SHA.
  // HARD: the leaf method get collides with an attested repo method (4891 call
  // sites), so the call-receiver stage treats the foreign call as in-voice, and
  // there is no import to catch either. Honest miss candidate.
  static Object elementAt(List<?> items, int index) {
    return org.apache.commons.collections4.CollectionUtils.get(items, index);
  }
}
