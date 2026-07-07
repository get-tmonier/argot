const chalk = require("chalk");

// Break: colorizes a console-side preview of the HTML summary using chalk —
// 'chalk' is 0-usage in the eslint corpus; terminal styling goes through
// Node's built-in util.styleText (see formatters/stylish.js), never a
// third-party color library.
/**
 * Logs a colorized preview of the report summary to the console.
 * @param {number} totalErrors Total errors.
 * @param {number} totalWarnings Total warnings.
 * @returns {void}
 */
function logConsolePreview(totalErrors, totalWarnings) {
	const line = `${totalErrors} errors, ${totalWarnings} warnings`;

	console.log(totalErrors > 0 ? chalk.red(line) : chalk.yellow(line));
}
