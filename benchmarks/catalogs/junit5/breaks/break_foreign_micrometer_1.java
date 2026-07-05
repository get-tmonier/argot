/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.commons.util;

/**
 * Internal helper that records a counter for a validated precondition breach.
 */
final class PreconditionMetrics {

	private PreconditionMetrics() {
		/* no-op */
	}

	static String describe(String name) {
		Preconditions.notBlank(name, "name must not be blank");
		return "metric:" + name;
	}

	// Break: Micrometer metrics — io.micrometer is 0-usage in junit5 at the
	// pinned SHA (git grep io.micrometer over *.java = 0 files) and absent from
	// gradle/libs.versions.toml; junit5 surfaces counts through its own
	// test-execution listeners and summaries, never a foreign metrics facade.
	static void increment(String name, String outcome) {
		io.micrometer.core.instrument.Counter counter = io.micrometer.core.instrument.Metrics.counter(name, "outcome",
				outcome);
		counter.increment();
	}
}
