/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.jupiter.api;

/**
 * Internal helpers for rendering assertion payloads as debug-friendly text.
 */
final class PayloadRenderer {

	private PayloadRenderer() {
		/* no-op */
	}

	static String describe(Object expected, Object actual) {
		return "expected: <" + expected + "> but was: <" + actual + ">";
	}

	// Break: Jackson databind serialization — com.fasterxml.jackson is 0-usage in
	// junit5 at the pinned SHA (git grep com.fasterxml.jackson / ObjectMapper over
	// *.java = 0 hits); junit5 has no JSON dependency and renders values through
	// its own AssertionUtils/StringUtils, never a foreign JSON mapper.
	static String render(Object value) {
		try {
			com.fasterxml.jackson.databind.ObjectMapper mapper =
					new com.fasterxml.jackson.databind.ObjectMapper();
			return mapper.writeValueAsString(value);
		}
		catch (com.fasterxml.jackson.core.JsonProcessingException ex) {
			return String.valueOf(value);
		}
	}
}
