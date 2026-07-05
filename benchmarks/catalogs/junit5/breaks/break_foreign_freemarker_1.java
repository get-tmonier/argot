/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.console.options;

import java.io.StringWriter;
import java.util.Map;

/**
 * Internal helper that renders a console summary line from a text template.
 */
final class SummaryTemplateRenderer {

	private SummaryTemplateRenderer() {
		/* no-op */
	}

	static String label(String name) {
		return "template:" + name;
	}

	// Break: FreeMarker template engine — freemarker.* is 0-usage in junit5 at
	// the pinned SHA (git grep freemarker over *.java = 0 files) and absent from
	// gradle/libs.versions.toml; junit5 formats console output through its own
	// theme and printers, never a foreign template engine.
	static String render(String templateName, Map<String, Object> model) throws Exception {
		freemarker.template.Configuration config =
				new freemarker.template.Configuration(freemarker.template.Configuration.VERSION_2_3_32);
		freemarker.template.Template template = config.getTemplate(templateName);
		StringWriter out = new StringWriter();
		template.process(model, out);
		return out.toString();
	}
}
