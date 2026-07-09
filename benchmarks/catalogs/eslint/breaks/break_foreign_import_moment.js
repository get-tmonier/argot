const moment = require("moment");

// Break: formats a "Generated at" timestamp banner ahead of the timing
// table print, imported at the top of the hunk. 'moment' is 0-usage in the
// eslint corpus and absent from package.json — the repo's own timing table
// (built via alignLeft/alignRight above) never prints a wall-clock
// timestamp, and nowhere else in lib/ is a date formatted through a date
// library; hrtime-based durations here are formatted with plain
// .toFixed(3). EASY: explicit foreign import, caught by the import stage.
/**
 * Builds a "Generated at" banner line for the timing table.
 * @returns {string} A formatted timestamp banner.
 */
function buildGeneratedAtBanner() {
	return `Generated at: ${moment().format("YYYY-MM-DD HH:mm:ss")}`;
}
