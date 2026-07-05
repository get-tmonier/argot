/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.engine.support.hierarchical;

import io.netty.channel.EventLoopGroup;
import io.netty.channel.nio.NioEventLoopGroup;
import io.netty.util.concurrent.Future;

/**
 * Internal helper that fans test tasks onto a Netty event loop group.
 */
final class NettyEventLoopExecutor {

	private final String name;

	NettyEventLoopExecutor(String name) {
		this.name = name;
	}

	String label() {
		return "loop:" + this.name;
	}

	// Break: Netty event loop — io.netty is 0-usage in junit5 at the pinned SHA
	// (git grep io.netty over *.java = 0 files) and absent from
	// gradle/libs.versions.toml; junit5 parallelises with a java.util.concurrent
	// ForkJoinPool through its own executor service, never a foreign event loop.
	void submitAll(Iterable<Runnable> tasks) {
		EventLoopGroup group = new NioEventLoopGroup(4);
		for (Runnable task : tasks) {
			Future<?> handle = group.submit(task);
			handle.awaitUninterruptibly();
		}
		group.shutdownGracefully();
	}
}
