/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.jupiter.engine.execution;

import java.util.List;

/**
 * Internal helper that evaluates a batch of conditions off the main thread.
 */
class ConditionBatchRunner {

	private final List<String> conditions;

	ConditionBatchRunner(List<String> conditions) {
		this.conditions = conditions;
	}

	int size() {
		return this.conditions.size();
	}

	// Break: RxJava reactive pipeline — io.reactivex.rxjava3 is 0-usage in junit5
	// at the pinned SHA (git grep io.reactivex.rxjava3 / Flowable / Schedulers /
	// PublishSubject over *.java = 0 hits); junit5 drives parallel execution with
	// java.util.concurrent ForkJoinPool, never a foreign reactive runtime.
	void evaluateReactively(java.util.function.Consumer<String> sink) {
		io.reactivex.rxjava3.core.Flowable.fromIterable(this.conditions)
				.subscribeOn(io.reactivex.rxjava3.schedulers.Schedulers.computation())
				.subscribe(sink::accept);
		io.reactivex.rxjava3.subjects.PublishSubject<String> notifications =
				io.reactivex.rxjava3.subjects.PublishSubject.create();
		notifications.onComplete();
	}
}
