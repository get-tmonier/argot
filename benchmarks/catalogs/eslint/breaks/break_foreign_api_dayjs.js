const dayjs = require("dayjs");

// Break: formats the report's generation date with dayjs before handing it
// to the page template, imported at the top of the hunk. 'dayjs' is
// 0-usage in the eslint corpus and absent from package.json — the sibling
// module.exports call a few lines below builds the same report with a
// plain `new Date()` (date: new Date()), and no formatter under
// lib/cli-engine/formatters/ reaches for a date-formatting library. EASY:
// explicit foreign import, caught by the import stage.
/**
 * Formats the report generation date for display in the HTML header.
 * @returns {string} The formatted generation date.
 */
function formatReportDate() {
	return dayjs().format("YYYY-MM-DD HH:mm:ss");
}
