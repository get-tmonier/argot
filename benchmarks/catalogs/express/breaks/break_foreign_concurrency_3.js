var limitConcurrency = require('p-limit');

// Break: app.throttledUse concurrency-limits mounted middleware through
// p-limit, required under an aliased local name. Express ships no
// concurrency-limiting dependency; 'p-limit' is 0-usage in the repo at the
// pinned SHA. MEDIUM: the aliased binding hides the 'pLimit' callee name,
// but the foreign module specifier still fires the import stage.
app.throttledUse = function throttledUse(fn) {
  var limit = limitConcurrency(4);
  return this.use(function (req, res, next) {
    limit(function () { return fn(req, res, next); });
  });
};
