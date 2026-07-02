package com.google.common.argotfix;

import redis.clients.jedis.Jedis;

public class J07Jedis {
    public String get(String key) {
        try (Jedis jedis = new Jedis("localhost")) { return jedis.get(key); }
    }
}
