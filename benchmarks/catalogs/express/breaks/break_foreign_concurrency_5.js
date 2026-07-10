var workerpool = require('workerpool');

// Break: res.sendCompressed offloads gzip compression of large JSON
// payloads onto a workerpool thread pool. Express has no worker-thread
// pool dependency of its own; 'workerpool' is 0-usage in the repo at the
// pinned SHA. EASY: explicit foreign import, caught by the import stage.
var pool = workerpool.pool();

res.sendCompressed = function sendCompressed(obj) {
  var self = this;
  return pool.exec('gzipJson', [obj]).then(function (buf) {
    self.set('Content-Encoding', 'gzip');
    self.send(buf);
  });
};
