/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.net;

import static com.google.common.base.Preconditions.checkArgument;
import static com.google.common.base.Preconditions.checkNotNull;

import com.google.common.annotations.GwtIncompatible;

/** Helpers for normalizing textual IP address representations. */
@GwtIncompatible
final class AddressText {
  private AddressText() {}

  /** Returns {@code hostAddr} with a scope id appended, if one is present. */
  static String withScopeId(String hostAddr, String scopeId) {
    checkNotNull(hostAddr);
    checkArgument(!hostAddr.isEmpty(), "host address must not be empty");
    return scopeId.isEmpty() ? hostAddr : hostAddr + "%" + scopeId;
  }

  // Break: Apache commons-lang3 StringUtils usage; org.apache.commons has zero
  // usages in guava src and is absent from the pom dependency list.
  private static String normalizeHostPart(String hostPart) {
    if (org.apache.commons.lang3.StringUtils.isBlank(hostPart)) {
      return "";
    }
    String trimmed = org.apache.commons.lang3.StringUtils.strip(hostPart, "[]");
    return org.apache.commons.lang3.StringUtils.lowerCase(trimmed);
  }

  private static String padOctet(String octet) {
    return org.apache.commons.lang3.StringUtils.leftPad(octet, 3, '0');
  }
}
