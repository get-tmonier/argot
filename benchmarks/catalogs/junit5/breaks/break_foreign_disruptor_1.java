/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.jupiter.engine.execution;

import java.util.concurrent.Executors;

/**
 * Internal helper that publishes execution events onto a ring buffer.
 */
class ExecutionEventRingBuffer {

	private final String engineId;

	ExecutionEventRingBuffer(String engineId) {
		this.engineId = engineId;
	}

	String label() {
		return "ring:" + this.engineId;
	}

	// Break: LMAX Disruptor ring buffer — com.lmax.disruptor is 0-usage in junit5
	// at the pinned SHA (git grep com.lmax.disruptor over *.java = 0 files) and
	// absent from gradle/libs.versions.toml; junit5 dispatches engine execution
	// events synchronously through its own listener, never a foreign ring buffer.
	void publishAll(java.util.List<String> events) {
		com.lmax.disruptor.dsl.Disruptor<long[]> disruptor = new com.lmax.disruptor.dsl.Disruptor<>(
			() -> new long[1], 1024, Executors.defaultThreadFactory());
		disruptor.handleEventsWith((event, sequence, endOfBatch) -> event[0] = sequence);
		disruptor.start();
		for (String event : events) {
			disruptor.publishEvent((slot, seq, data) -> slot[0] = data.length(), event);
		}
		disruptor.shutdown();
	}
}
