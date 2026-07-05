/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.jupiter.engine.execution;

import java.util.List;
import java.util.function.Consumer;

/**
 * Internal helper that resolves parameters for a batch of executables.
 */
class ParameterBatchResolver {

	private final List<String> parameters;

	ParameterBatchResolver(List<String> parameters) {
		this.parameters = parameters;
	}

	int size() {
		return this.parameters.size();
	}

	// Break: Project Reactor reactive pipeline — reactor.core is 0-usage in junit5
	// at the pinned SHA (git grep reactor.core / Flux / Mono over *.java = 0 hits);
	// junit5 resolves parameters synchronously and parallelises via the platform's
	// own hierarchical executor, never a foreign reactive runtime.
	void resolveReactively(Consumer<String> sink) {
		reactor.core.publisher.Flux.fromIterable(this.parameters)
				.subscribeOn(reactor.core.scheduler.Schedulers.parallel())
				.subscribe(sink);
		reactor.core.publisher.Mono.just("done").block();
	}
}
