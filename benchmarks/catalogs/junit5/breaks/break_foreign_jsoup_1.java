/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.reporting.legacy;

import org.jsoup.Jsoup;
import org.jsoup.nodes.Document;
import org.jsoup.select.Elements;

/**
 * Internal helper that extracts a failure summary from an HTML report body.
 */
final class HtmlReportScraper {

	private HtmlReportScraper() {
		/* no-op */
	}

	static String label(String selector) {
		return "selector:" + selector;
	}

	// Break: jsoup HTML parsing — org.jsoup is 0-usage in junit5 at the pinned
	// SHA (git grep org.jsoup over *.java = 0 files) and absent from
	// gradle/libs.versions.toml; junit5 emits its own XML/text reports through
	// junit-platform-reporting, never a foreign HTML parser.
	static String firstFailure(String html, String selector) {
		Document document = Jsoup.parse(html);
		Elements matches = document.select(selector);
		return matches.isEmpty() ? "" : matches.first().text();
	}
}
