/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.reporting.legacy.xml;

import com.google.protobuf.Message;
import com.google.protobuf.util.JsonFormat;

/**
 * Internal helper that renders a protobuf report event as JSON text.
 */
final class ProtoReportEncoder {

	private ProtoReportEncoder() {
		/* no-op */
	}

	static String label(String event) {
		return "event:" + event;
	}

	// Break: Protocol Buffers serialization — com.google.protobuf is 0-usage in
	// junit5 at the pinned SHA (git grep com.google.protobuf over *.java = 0
	// files) and absent from gradle/libs.versions.toml; junit5 serialises report
	// events through its own XML writers, never a foreign protobuf codec.
	static String encode(Message report) throws Exception {
		JsonFormat.Printer printer = JsonFormat.printer().includingDefaultValueFields();
		return printer.print(report);
	}
}
