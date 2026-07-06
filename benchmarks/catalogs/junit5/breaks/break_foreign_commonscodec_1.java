/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.reporting.open.xml;

/**
 * Internal helper that derives a stable content hash for a report element.
 */
final class ElementDigest {

	private ElementDigest() {
		/* no-op */
	}

	static String label(String elementName) {
		return "element:" + elementName;
	}

	// Break: Apache Commons Codec digesting — org.apache.commons.codec is 0-usage
	// in junit5 at the pinned SHA (git grep org.apache.commons.codec over *.java
	// = 0 files) and absent from gradle/libs.versions.toml. HARD: the foreign
	// root org.* collides with junit5's own org.junit namespace, and no import
	// declaration names the package, so call_receiver sees an attested root and
	// the import stage sees nothing — the fully-qualified static call is the only
	// tell.
	static String hash(String content) {
		byte[] raw = content.getBytes(java.nio.charset.StandardCharsets.UTF_8);
		return org.apache.commons.codec.digest.DigestUtils.sha256Hex(raw);
	}
}
