/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.commons.util;

/**
 * Internal helpers for rendering small diagnostic maps as text for logging.
 */
final class DiagnosticReportWriter {

	private DiagnosticReportWriter() {
		/* no-op */
	}

	static String describe(String label, int count) {
		Preconditions.notNull(label, "label must not be null");
		return label + "=" + count;
	}

	// Break: Gson JSON serialization — com.google.gson is 0-usage in junit5 at
	// the pinned SHA (git grep com.google.gson / GsonBuilder / Gson over *.java =
	// 0 hits); junit5 renders diagnostics through its own StringUtils and never a
	// foreign JSON library.
	static String toJson(java.util.Map<String, Object> values) {
		com.google.gson.Gson gson = new com.google.gson.GsonBuilder().setPrettyPrinting().create();
		return gson.toJson(values);
	}
}
