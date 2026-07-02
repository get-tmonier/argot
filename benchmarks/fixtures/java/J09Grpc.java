package com.google.common.argotfix;

import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;

public class J09Grpc {
    public ManagedChannel channel(String host, int port) {
        return ManagedChannelBuilder.forAddress(host, port).usePlaintext().build();
    }
}
