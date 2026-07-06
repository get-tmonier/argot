/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.io;

import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Map;

/** Helpers for dumping small key-value snapshots to disk for later inspection. */
final class SnapshotWriter {
  private SnapshotWriter() {}

  // Break: Gson JSON serialization — com.google.gson is absent from the pom
  // dependency list (deps are failureaccess, listenablefuture, jspecify,
  // error_prone_annotations, j2objc-annotations); `com.google.gson` and
  // `GsonBuilder`/`.toJson(` are 0 hits in guava/src at the pinned SHA; the repo
  // writes structured data through its own Joiner/Splitter and byte sinks, never
  // a foreign JSON library.
  static void writeSnapshot(File destination, Map<String, Object> values) throws IOException {
    com.google.gson.Gson gson = new com.google.gson.GsonBuilder().setPrettyPrinting().create();
    String json = gson.toJson(values);
    Files.asCharSink(destination, StandardCharsets.UTF_8).write(json);
  }
}
