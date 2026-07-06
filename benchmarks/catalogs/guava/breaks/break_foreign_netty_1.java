/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.util.concurrent;

import java.util.concurrent.ExecutorService;
import io.netty.channel.nio.NioEventLoopGroup;

/** Helpers for provisioning a non-blocking event loop. */
final class EventLoops {
  private EventLoops() {}

  // Break: Netty NioEventLoopGroup — io.netty is absent from the pom dependency
  // list and has 0 usages in guava/src at the pinned SHA; guava provisions
  // executors through its own factories, never a foreign NIO event loop.
  static ExecutorService newEventLoop(int threads) {
    return new NioEventLoopGroup(threads);
  }
}
