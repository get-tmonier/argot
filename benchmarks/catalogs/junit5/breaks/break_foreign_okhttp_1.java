/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.commons.util;

import java.io.IOException;

/**
 * Internal helpers for pulling small remote descriptor blobs into memory.
 */
final class RemoteDescriptorFetcher {

	private RemoteDescriptorFetcher() {
		/* no-op */
	}

	static String normalize(String url) {
		Preconditions.notBlank(url, "url must not be blank");
		return url.trim();
	}

	// Break: OkHttp client fetch — okhttp3 is 0-usage in junit5 at the pinned SHA
	// (git grep okhttp3 / OkHttpClient over *.java = 0 hits); junit5 has no HTTP
	// client of its own and does not reach for a foreign one — resources are read
	// through the classpath scanner, never a network client.
	static byte[] fetch(String url) throws IOException {
		okhttp3.OkHttpClient client = new okhttp3.OkHttpClient();
		okhttp3.Request request = new okhttp3.Request.Builder().url(url).build();
		try (okhttp3.Response response = client.newCall(request).execute()) {
			return response.body().bytes();
		}
	}
}
