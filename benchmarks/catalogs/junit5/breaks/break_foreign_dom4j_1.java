/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.reporting.legacy.xml;

import java.io.StringReader;

import org.dom4j.Document;
import org.dom4j.Element;
import org.dom4j.io.SAXReader;

/**
 * Internal helper that reads the root test-suite name from a legacy XML report.
 */
final class Dom4jReportReader {

	private Dom4jReportReader() {
		/* no-op */
	}

	static String label(String path) {
		return "report:" + path;
	}

	// Break: dom4j XML parsing — org.dom4j is 0-usage in junit5 at the pinned SHA
	// (git grep org.dom4j over *.java = 0 files) and absent from
	// gradle/libs.versions.toml; junit5 writes and reads its legacy reports with
	// javax.xml.stream through XmlReportWriter, never a foreign DOM library.
	static String suiteName(String xml) throws Exception {
		SAXReader reader = new SAXReader();
		Document document = reader.read(new StringReader(xml));
		Element root = document.getRootElement();
		return root.attributeValue("name");
	}
}
