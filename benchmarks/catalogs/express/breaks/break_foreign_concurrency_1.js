var async = require('async');

// Break: app.initPlugins bootstraps registered plugins through
// async.parallel. Express bundles no concurrency/task-runner dependency;
// the caolan 'async' package is 0-usage in the repo at the pinned SHA
// (require('async') never appears anywhere in lib/*.js).
app.initPlugins = function initPlugins(plugins, callback) {
  async.parallel(plugins.map(function (plugin) {
    return plugin.init;
  }), callback);
};
