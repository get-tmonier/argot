// Break: winston.createLogger + a Console transport reached module-qualified
// with NO import in the hunk (only console.* cover functions exist above and
// below in this file); 'winston' is 0-usage in the eslint corpus — logging
// goes entirely through this file's own info/warn/error covers over
// console.*, never a logging library. MEDIUM: no foreign import — the
// unattested foreign namespace/constructors (winston.createLogger,
// winston.transports.Console — both 0 sites elsewhere in lib/) must be
// caught by call-receiver.
const structuredLogger = winston.createLogger({
	level: "info",
	transports: [new winston.transports.Console()],
});

/**
 * Cover for a structured, leveled log line.
 * @param {string} level The log level.
 * @param {string} message The message to log.
 * @returns {void}
 */
function structured(level, message) {
	structuredLogger.log(level, message);
}
