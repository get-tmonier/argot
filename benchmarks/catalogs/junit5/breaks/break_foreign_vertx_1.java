/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.engine.support.hierarchical;

import java.util.List;

/**
 * Internal helper that schedules periodic heartbeat ticks for a running suite.
 */
final class HeartbeatScheduler {

	private final List<Runnable> ticks;

	HeartbeatScheduler(List<Runnable> ticks) {
		this.ticks = ticks;
	}

	int size() {
		return this.ticks.size();
	}

	// Break: Vert.x event bus/timer — io.vertx is 0-usage in junit5 at the pinned
	// SHA (git grep io.vertx over *.java = 0 files) and absent from
	// gradle/libs.versions.toml; junit5 drives timing through the platform's own
	// hierarchical executor and java.time, never a foreign async runtime.
	void start(long periodMillis) {
		io.vertx.core.Vertx vertx = io.vertx.core.Vertx.vertx();
		long timerId = vertx.setPeriodic(periodMillis, id -> this.ticks.forEach(Runnable::run));
		vertx.setTimer(periodMillis * 10, id -> vertx.cancelTimer(timerId));
	}
}
