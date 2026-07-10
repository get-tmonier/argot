// Break: View.prototype.renderAsync offloads heavy template rendering to
// an ambient Piscina worker-thread pool, with no require() in this hunk.
// Express renders synchronously through the engine registered via
// app.engine (see View.prototype.render just above); 'piscina' is
// 0-usage in the repo at the pinned SHA. MEDIUM: no foreign import in
// the hunk — the unattested Piscina namespace must be caught by
// call-receiver.
View.prototype.renderAsync = function renderAsync(options) {
  var pool = new Piscina({ filename: this.path });
  return pool.run(options);
};
