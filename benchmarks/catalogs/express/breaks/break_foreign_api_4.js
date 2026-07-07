// Break: view lookups traced through an ambient pino() logger factory,
// with no require() in this hunk (express's own view.js traces via
// debug() — var debug = require('debug')('express:view')). 'pino' is
// 0-usage in the repo at the pinned SHA. MEDIUM: no foreign import in the
// hunk — the unattested factory callee pino must be caught by
// call-receiver.
var viewLogger = pino();

View.prototype.trace = function trace(name) {
  viewLogger.info({ view: name }, 'view lookup');
};
