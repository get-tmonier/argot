/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.io;

import java.io.IOException;

/** Helpers for pulling small remote resources into byte form. */
final class RemoteResourceFetcher {
  private RemoteResourceFetcher() {}

  // Break: OkHttp client fetch — okhttp3 is absent from the pom dependency list
  // (deps are failureaccess, listenablefuture, jspecify, error_prone_annotations,
  // j2objc-annotations); `okhttp3` and `OkHttpClient`/`.newCall(` are 0 hits in
  // guava/src at the pinned SHA; the repo has no HTTP client of its own and does
  // not reach for a foreign one — resource loading goes through Resources/Files.
  static byte[] fetch(String url) throws IOException {
    okhttp3.OkHttpClient client = new okhttp3.OkHttpClient();
    okhttp3.Request request = new okhttp3.Request.Builder().url(url).build();
    try (okhttp3.Response response = client.newCall(request).execute()) {
      return response.body().bytes();
    }
  }
}
