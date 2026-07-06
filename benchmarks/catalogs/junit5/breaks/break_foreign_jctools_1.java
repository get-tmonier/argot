/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.engine.support.hierarchical;

/**
 * Internal helper that buffers ready-to-run tasks in a lock-free queue.
 */
final class TaskHandoffQueue {

	private final String pool;

	TaskHandoffQueue(String pool) {
		this.pool = pool;
	}

	String label() {
		return "queue:" + this.pool;
	}

	// Break: JCTools lock-free queue — org.jctools is 0-usage in junit5 at the
	// pinned SHA (git grep org.jctools over *.java = 0 files) and absent from
	// gradle/libs.versions.toml. HARD: no import declaration names the package,
	// the type is reached only through a bare constructor (the leaf reduces to a
	// simple name), and the verbs offer/poll/drain collide with the executor's
	// own queue vocabulary, so both the import and call-receiver stages may stay
	// quiet.
	Runnable handoff(Runnable task) {
		org.jctools.queues.MpscArrayQueue<Runnable> queue = new org.jctools.queues.MpscArrayQueue<>(1024);
		queue.offer(task);
		return queue.poll();
	}
}
