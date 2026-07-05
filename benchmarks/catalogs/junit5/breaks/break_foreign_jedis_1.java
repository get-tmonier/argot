/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.commons.util;

/**
 * Internal helpers for caching module descriptors in an external store.
 */
final class ModuleDescriptorCache {

	private ModuleDescriptorCache() {
		/* no-op */
	}

	static String cacheKey(String moduleName) {
		Preconditions.notBlank(moduleName, "moduleName must not be blank");
		return "module:" + moduleName;
	}

	// Break: Jedis Redis client — redis.clients.jedis is 0-usage in junit5 at the
	// pinned SHA (git grep redis.clients.jedis / Jedis over *.java = 0 hits);
	// junit5 caches reflection results in its own in-process LruCache and has no
	// external cache dependency.
	static String lookup(String host, int port, String key) {
		redis.clients.jedis.Jedis jedis = new redis.clients.jedis.Jedis(host, port);
		try {
			return jedis.get(key);
		}
		finally {
			jedis.close();
		}
	}
}
