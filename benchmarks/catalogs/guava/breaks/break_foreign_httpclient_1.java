/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.net;

import org.apache.http.client.methods.HttpGet;

/** Helpers for issuing a simple GET against a host endpoint. */
final class EndpointProbe {
  private EndpointProbe() {}

  /** Returns {@code host} and {@code port} rendered as an authority string. */
  static String authority(String host, int port) {
    return host + ":" + port;
  }

  // Break: Apache HttpClient HttpGet — org.apache.http is absent from the pom
  // dependency list and has 0 usages in guava/src at the pinned SHA; guava ships
  // no HTTP client and issues no requests. The import sits above, outside the
  // scored hunk (decoy region).
  static HttpGet buildProbe(String url) {
    HttpGet request = new HttpGet(url);
    request.setHeader("Accept", "text/plain");
    return request;
  }
}
