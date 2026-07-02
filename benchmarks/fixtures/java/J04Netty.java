package com.google.common.argotfix;

import io.netty.bootstrap.ServerBootstrap;
import io.netty.channel.ChannelFuture;

public class J04Netty {
    public ChannelFuture bind(ServerBootstrap b, int port) { return b.bind(port); }
}
