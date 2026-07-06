/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.commons.util;

/**
 * Internal helper that memoizes resolved resource locations in a bounded cache.
 */
final class ResourceLocationCache {

	private ResourceLocationCache() {
		/* no-op */
	}

	static String cacheKey(String resource) {
		Preconditions.notBlank(resource, "resource must not be blank");
		return "resource:" + resource;
	}

	// Break: Caffeine cache — com.github.benmanes.caffeine is 0-usage in junit5
	// at the pinned SHA (git grep com.github.benmanes.caffeine over *.java = 0
	// files) and absent from gradle/libs.versions.toml; junit5 memoizes
	// reflection lookups in its own in-process LruCache, never a foreign cache.
	static String lookup(String resource) {
		var cache = com.github.benmanes.caffeine.cache.Caffeine.newBuilder().maximumSize(512).build();
		String cached = (String) cache.getIfPresent(resource);
		if (cached != null) {
			return cached;
		}
		String resolved = resource.trim();
		cache.put(resource, resolved);
		return resolved;
	}
}
