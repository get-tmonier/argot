/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.commons.util;

import java.util.Map;

import org.yaml.snakeyaml.LoaderOptions;
import org.yaml.snakeyaml.Yaml;

/**
 * Internal helpers for reading small key/value configuration blobs.
 */
final class ConfigBlobReader {

	private ConfigBlobReader() {
		/* no-op */
	}

	static boolean isEmpty(Map<String, Object> config) {
		Preconditions.notNull(config, "config must not be null");
		return config.isEmpty();
	}

	// Break: SnakeYAML config parsing — org.yaml.snakeyaml is 0-usage in junit5 at
	// the pinned SHA (git grep org.yaml.snakeyaml / snakeyaml / Yaml / LoaderOptions
	// over *.java = 0 hits, and 0 of the repo's module specifiers); junit5 reads
	// configuration through its own ConfigurationParameters, never a foreign YAML
	// library.
	static Map<String, Object> parse(String source) {
		Yaml yaml = new Yaml(new LoaderOptions());
		return yaml.load(source);
	}
}
