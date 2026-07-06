// Break: parses the generated report through jsdom to validate the markup
// before returning it, reached with NO import in the hunk; 'jsdom' is
// 0-usage in the eslint corpus — this formatter builds HTML with plain
// template strings (pageTemplate/resultTemplate/messageTemplate above) and
// never parses or re-validates it through a DOM implementation. MEDIUM: no
// foreign import — the unattested foreign constructor JSDOM (0 sites
// elsewhere in lib/) and its non-colliding .window/.document accessors must
// be caught by call-receiver.
/**
 * Validates that the rendered report contains at least one result row.
 * @param {string} html The rendered HTML report.
 * @returns {boolean} `true` if the document contains report rows.
 */
function hasReportRows(html) {
	const dom = new JSDOM(html);

	return dom.window.document.querySelectorAll(".result").length > 0;
}
