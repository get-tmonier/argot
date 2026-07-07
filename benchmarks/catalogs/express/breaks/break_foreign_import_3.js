var merge = require('lodash/merge');

// Break: mergeSettings pulls in the lodash 'merge' submodule where the
// codebase already composes settings via object spread (see
// application.js's app.render: `{ ...this.locals, ...opts._locals, ...opts }`).
// 'lodash' is 0-usage in the repo at the pinned SHA. MEDIUM: submodule
// import path, but the foreign module specifier still fires the import
// stage.
exports.mergeSettings = function mergeSettings(base, overrides) {
  return merge({}, base, overrides);
};
