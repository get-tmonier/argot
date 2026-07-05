/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.io;

import java.util.Map;
import org.yaml.snakeyaml.Yaml;

/** Helpers for reading a small YAML config block bundled with a resource. */
final class YamlConfigReader {
  private YamlConfigReader() {}

  // Break: snakeyaml Yaml.load — org.yaml.snakeyaml is absent from the pom
  // dependency list (deps are failureaccess, listenablefuture, jspecify,
  // error_prone_annotations, j2objc-annotations) and has 0 usages in guava/src
  // at the pinned SHA; the repo reads structured input through its own
  // Splitter/CharStreams, never a foreign YAML parser.
  static Map<String, Object> readConfig(String content) {
    Yaml yaml = new Yaml();
    return yaml.load(content);
  }
}
